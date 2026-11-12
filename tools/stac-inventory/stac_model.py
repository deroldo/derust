"""Extracts the Stac JSON model (enums, classes, fields) from `packages/stac_core/lib`.

Python 3 stdlib only. The Dart sources are very regular (json_serializable), so a
small purpose-built reader is enough. The generated `*.g.dart` files are used as
the source of truth for the JSON keys of each class (`_$XToJson`).
"""

import os
import re

SKIP_FILES = {
    "widgets.dart", "actions.dart", "foundation.dart", "theme.dart", "core.dart",
    "annotations.dart", "stac_core.dart", "stac_icons.dart",
}


def strip_comments(text):
    """Removes // and /* */ comments (doc comments included), keeping line structure."""
    out = []
    i, n = 0, len(text)
    in_str = None
    while i < n:
        c = text[i]
        if in_str:
            out.append(c)
            if c == "\\":
                out.append(text[i + 1])
                i += 2
                continue
            if c == in_str:
                in_str = None
            i += 1
            continue
        if c in "'\"":
            in_str = c
            out.append(c)
            i += 1
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("\n" * text.count("\n", i, j))
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def match_brace(text, open_idx):
    depth = 0
    i = open_idx
    in_str = None
    while i < len(text):
        c = text[i]
        if in_str:
            if c == "\\":
                i += 2
                continue
            if c == in_str:
                in_str = None
        elif c in "'\"":
            in_str = c
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced braces")


def doc_before(text, idx):
    """Doc comment (`///`) block immediately preceding position idx, as a list of lines."""
    lines = text[:idx].rstrip("\n").split("\n")
    doc = []
    # skip annotations directly above the declaration
    k = len(lines) - 1
    while k >= 0 and (lines[k].strip().startswith("@") or lines[k].strip() == ""):
        k -= 1
    while k >= 0 and lines[k].strip().startswith("///"):
        doc.append(re.sub(r"^\s*/// ?", "", lines[k]))
        k -= 1
    return list(reversed(doc))


ENUM_RE = re.compile(r"^enum\s+(\w+)\s*\{", re.M)
CLASS_RE = re.compile(
    r"^(abstract\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([\w,\s<>?]+?))?\s*\{",
    re.M,
)
NON_FINAL_FIELD_RE = re.compile(
    r"^  (?:final\s+)?((?:[A-Z]\w*|bool|int|double|dynamic)(?:<[^;()]*>)?\??)\s+(\w+)\s*;", re.M
)
NON_FINAL_CLASSES = {"StacCustomTextStyle"}
FIELD_RE = re.compile(
    r"^  (?:@\w+(?:\([^)]*\))?\s+)*final\s+(.+?)\s+(\w+)\s*;", re.M
)


def parse_file(path, rel):
    raw = open(path, encoding="utf-8").read()
    clean = strip_comments(raw)
    enums, classes = [], []
    for m in ENUM_RE.finditer(clean):
        end = match_brace(clean, m.end() - 1)
        body = clean[m.end():end]
        values = [v.strip() for v in body.split(",") if v.strip()]
        enums.append({
            "name": m.group(1),
            "values": values,
            "dart_path": rel,
            "doc": doc_before(raw, raw.find(m.group(0))),
        })
    for m in CLASS_RE.finditer(clean):
        end = match_brace(clean, m.end() - 1)
        body = clean[m.end():end]
        name = m.group(2)
        decl_pos = raw.find(m.group(0).split("{")[0])
        header_has_json = "@JsonSerializable" in clean[max(0, m.start() - 80):m.start()]
        # constructor parameters of the primary constructor
        ctor = re.search(r"(?:const\s+)?%s\s*\(\s*\{(.*?)\}\s*\)" % name, body, re.S)
        ctor_text = ctor.group(1) if ctor else ""
        required = set(re.findall(r"required\s+(?:this\.)?(\w+)", ctor_text))
        defaults = set(re.findall(r"this\.(\w+)\s*=", ctor_text))
        ctor_fields = set(re.findall(r"this\.(\w+)", ctor_text))
        fields = []
        field_re = NON_FINAL_FIELD_RE if name in NON_FINAL_CLASSES else FIELD_RE
        for fm in field_re.finditer(body):
            ftype, fname = fm.group(1).strip(), fm.group(2)
            line_start = body.rfind("\n", 0, fm.start()) + 1
            annotations = body[max(0, line_start - 200):fm.end()]
            fields.append({
                "name": fname,
                "dart_type": ftype,
                "nullable": ftype.endswith("?"),
                "required": fname in required,
                "has_default": fname in defaults,
                "in_ctor": fname in ctor_fields,
                "double_converter": "@DoubleConverter" in fm.group(0)
                or bool(re.search(r"@DoubleConverter\(\)\s*(?:[\w<>?,\s]*\s+)?%s\b" % fname, ctor_text)),
            })
        # discriminator getters
        disc = None
        dm = re.search(r"String\s+get\s+(type|actionType)\s*=>\s*(.+?);", body, re.S)
        if dm:
            disc = {"key": dm.group(1), "expr": dm.group(2).strip()}
        sm = re.search(r"super(?:\._)?\(\s*type:\s*(\w+)\.(\w+)\s*\)", body)
        classes.append({
            "name": name,
            "super_type": {"enum": sm.group(1), "value": sm.group(2)} if sm else None,
            "abstract": bool(m.group(1)),
            "extends": m.group(3),
            "implements": [s.strip() for s in (m.group(4) or "").split(",") if s.strip()],
            "dart_path": rel,
            "json_serializable": header_has_json,
            "fields": fields,
            "discriminator_getter": disc,
            "doc": doc_before(raw, decl_pos),
        })
    return enums, classes


def field_docs(path):
    """Maps field name -> doc lines for a Dart file (best-effort)."""
    raw = open(path, encoding="utf-8").read()
    lines = raw.split("\n")
    docs = {}
    for idx, line in enumerate(lines):
        m = re.match(r"^  (?:@\w+(?:\([^)]*\))?\s+)*final\s+.+?\s+(\w+)\s*;", line)
        if not m:
            continue
        k = idx - 1
        d = []
        while k >= 0 and (lines[k].strip().startswith("///") or lines[k].strip().startswith("@")):
            if lines[k].strip().startswith("///"):
                d.append(re.sub(r"^\s*/// ?", "", lines[k]))
            k -= 1
        docs.setdefault(m.group(1), list(reversed(d)))
    return docs


TOJSON_RE = re.compile(
    r"Map<String, dynamic>\s+_\$(\w+)ToJson\(\s*\w+\s+instance\s*,?\s*\)\s*(?:=>|\{)\s*(?:return\s+)?<String, dynamic>\s*\{",
    re.S,
)


def parse_g(path):
    """Returns {ClassName: [json keys in order]} from a *.g.dart file."""
    text = open(path, encoding="utf-8").read()
    result = {}
    for m in TOJSON_RE.finditer(text):
        end = match_brace(text, m.end() - 1)
        body = text[m.end():end]
        keys = []
        depth = 0
        i = 0
        # top-level keys only: scan for `'key':` at nesting depth 0
        for km in re.finditer(r"'(\w+)'\s*:", body):
            pre = body[:km.start()]
            depth = (pre.count("(") + pre.count("[") + pre.count("{")) - (
                pre.count(")") + pre.count("]") + pre.count("}"))
            if depth == 0:
                keys.append(km.group(1))
        result[m.group(1)] = keys
    return result


ENUM_MAP_RE = re.compile(r"const _\$(\w+)EnumMap\s*=\s*\{(.*?)\};", re.S)


def parse_enum_maps(path):
    """Returns {EnumName: {dart_value: json_value}} from a *.g.dart file."""
    text = open(path, encoding="utf-8").read()
    maps = {}
    for m in ENUM_MAP_RE.finditer(text):
        entries = re.findall(r"\w+\.(\w+)\s*:\s*'([^']*)'", m.group(2))
        maps[m.group(1)] = dict(entries)
    return maps


def load(lib_root):
    enums, classes, gkeys, enum_maps = [], [], {}, {}
    for dirpath, _, files in sorted(os.walk(lib_root)):
        for f in sorted(files):
            if not f.endswith(".dart") or f in SKIP_FILES:
                continue
            full = os.path.join(dirpath, f)
            rel = os.path.relpath(full, lib_root)
            if f.endswith(".g.dart"):
                gkeys.update(parse_g(full))
                enum_maps.update(parse_enum_maps(full))
                continue
            e, c = parse_file(full, rel)
            fd = field_docs(full)
            for cl in c:
                for fld in cl["fields"]:
                    fld["doc"] = fd.get(fld["name"], [])
            enums.extend(e)
            classes.extend(c)
    unique = {}
    for e in enums:
        prev = unique.get(e["name"])
        if prev is None:
            unique[e["name"]] = e
        elif prev["values"] != e["values"]:
            raise ValueError("enum %s defined twice with different values" % e["name"])
        else:
            prev.setdefault("also_in", []).append(e["dart_path"])
    enums = list(unique.values())
    for c in classes:
        c["json_keys"] = gkeys.get(c["name"])
    for e in enums:
        m = enum_maps.get(e["name"])
        # json_serializable maps every used enum; unused ones fall back to the Dart name
        e["json_values"] = [m.get(v, v) if m else v for v in e["values"]]
        e["json_map_seen"] = m is not None
    return {"enums": enums, "classes": classes}


if __name__ == "__main__":
    import json
    import sys
    json.dump(load(sys.argv[1]), sys.stdout, indent=1)
