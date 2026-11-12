"""Fixture + test generation for gen_rust.py.

For every modeled component a JSON fixture is produced:
  * the "JSON Example" of the Dart docstring when it conforms to the model (unknown keys,
    wrong types or invalid enum values make it fall back), or
  * a synthetic sample populating every field (nested levels beyond depth 2 only carry
    required fields).
A Rust test builds the same value with the builders and compares it to the fixture.
"""

import json
import os
import re

from stac_catalog import snake, pascal

TOP_DIR = {"widget": "widgets", "action": "actions", "foundation": "foundation", "theme": "theme"}
POLY = {"StacWidget": "Widget", "StacAction": "Action", "StacTextStyle": "TextStyle",
        "StacShapeBorder": "ShapeBorder"}


class Mismatch(Exception):
    pass


def rust_str(s):
    return json.dumps(s, ensure_ascii=False)


def rust_float(x):
    r = repr(float(x))
    if "e" in r or "inf" in r or "nan" in r:
        raise Mismatch("unsupported float " + r)
    return r if "." in r else r + ".0"


class FixtureBuilder:
    def __init__(self, gen):
        self.g = gen
        self.members = {
            "Widget": {c["disc"][1]: c for c in gen.comps.values() if c.get("variant_of") == "Widget"},
            "Action": {c["disc"][1]: c for c in gen.comps.values() if c.get("variant_of") == "Action"},
            "ShapeBorder": {c["disc"][1]: c for c in gen.comps.values() if c.get("variant_of") == "ShapeBorder"},
            "TextStyle": {c["disc"][1]: c for c in gen.comps.values() if c.get("variant_of") == "TextStyle"},
        }
        self.by_rust = {c["rust"]: c for c in gen.comps.values()}

    # ------------------------------------------------------------------ docs
    def doc_example(self, comp):
        raw = self.g.classes[comp["dart"]]["doc"]
        text = "\n".join(raw)
        blocks = re.findall(r"```json\n(.*?)```", text, re.S)
        for b in blocks:
            try:
                return json.loads(b)
            except ValueError:
                continue
        return None

    # ------------------------------------------------------------------ conform
    def comp_of(self, dart):
        return self.g.comps[dart]

    def conform(self, v, ast, where=""):
        k = ast[0]
        if k == "prim":
            t = ast[1]
            if t == "String" and isinstance(v, str):
                return v
            if t == "double" and isinstance(v, (int, float)) and not isinstance(v, bool):
                return v
            if t == "int" and isinstance(v, int) and not isinstance(v, bool):
                return v
            if t == "bool" and isinstance(v, bool):
                return v
            raise Mismatch("%s: expected %s got %r" % (where, t, v))
        if k == "json":
            return v
        if k == "map":
            if isinstance(v, dict):
                return v
            raise Mismatch("%s: expected map" % where)
        if k == "imap":
            if not isinstance(v, dict):
                raise Mismatch("%s: expected map" % where)
            return {key: self.conform(val, ast[1], where + "." + key) for key, val in v.items()}
        if k == "list":
            if not isinstance(v, list):
                raise Mismatch("%s: expected list" % where)
            return [self.conform(x, ast[1], where + "[]") for x in v]
        name = ast[1]
        if name == "StateEntry":
            if isinstance(v, dict) and set(v) == {"key", "value"} and isinstance(v["key"], str):
                return v
            raise Mismatch("%s: bad state entry" % where)
        if name == "StacColor":
            if isinstance(v, str):
                return v
            raise Mismatch("%s: color must be string" % where)
        if name in self.g.enums:
            if v in self.g.enums[name]["json_values"]:
                return v
            raise Mismatch("%s: %r not in enum %s" % (where, v, name))
        if name in POLY:
            return self.conform_poly(v, POLY[name], where)
        return self.conform_class(v, self.comp_of(name), where)

    def conform_poly(self, v, base, where):
        if base == "TextStyle":
            if isinstance(v, str):
                v = {"type": "theme", "textTheme": v}
            elif isinstance(v, dict) and "type" not in v:
                v = dict(v, type="custom")
        if not isinstance(v, dict):
            raise Mismatch("%s: expected object for %s" % (where, base))
        key = "actionType" if base == "Action" else "type"
        t = v.get(key)
        if t not in self.members[base]:
            raise Mismatch("%s: unknown %s %s=%r" % (where, base, key, t))
        return self.conform_class(v, self.members[base][t], where)

    def conform_class(self, v, comp, where):
        if comp["dart"] == "StacEdgeInsets":
            if isinstance(v, (int, float)) and not isinstance(v, bool):
                v = {"left": v, "top": v, "right": v, "bottom": v}
            elif isinstance(v, list) and len(v) == 4:
                v = dict(zip(["left", "top", "right", "bottom"], v))
        if comp["dart"] == "StacBorderRadius":
            if isinstance(v, (int, float)) and not isinstance(v, bool):
                v = {"topLeft": v, "topRight": v, "bottomLeft": v, "bottomRight": v}
        if not isinstance(v, dict):
            raise Mismatch("%s: expected object for %s" % (where, comp["dart"]))
        fields = {f["json"]: f for f in comp["fields"]}
        out = {}
        disc = comp["disc"]
        for key, val in v.items():
            if disc and key == disc[0]:
                if val != disc[1]:
                    raise Mismatch("%s: %s=%r != %r" % (where, key, val, disc[1]))
                continue
            if key not in fields:
                raise Mismatch("%s: unknown key %r in %s" % (where, key, comp["dart"]))
            f = fields[key]
            if val is None:
                if f["opt"]:
                    continue
                raise Mismatch("%s: null for required %s" % (where, key))
            out[key] = self.conform(val, f["ast"], "%s.%s" % (where, key))
        for f in comp["fields"]:
            if not f["opt"] and f["json"] not in out:
                raise Mismatch("%s: missing required %s of %s" % (where, f["json"], comp["dart"]))
        if disc:
            out = dict([(disc[0], disc[1])] + list(out.items()))
        return out

    # ------------------------------------------------------------------ samples
    def sample_widget(self):
        return {"type": "text", "data": "sample"}

    def sample_action(self):
        return {"actionType": "navigate", "routeName": "/sample"}

    def sample(self, ast, name, depth):
        k = ast[0]
        if k == "prim":
            t = ast[1]
            if t == "String":
                return "#FF0000" if name.lower().endswith("color") else "sample"
            return {"double": 1.5, "int": 1, "bool": True}[t]
        if k == "json":
            return "sample"
        if k == "map":
            return {"key": "value"}
        if k == "imap":
            return {"0": self.sample(ast[1], name, depth + 1)}
        if k == "list":
            return [self.sample(ast[1], name, depth)]
        n = ast[1]
        if n == "StateEntry":
            return {"key": "sample", "value": "value"}
        if n == "StacColor":
            return "#FF0000"
        if n in self.g.enums:
            return self.g.enums[n]["json_values"][0]
        if n == "StacWidget":
            return self.sample_widget()
        if n == "StacAction":
            return self.sample_action()
        if n == "StacTextStyle":
            return {"type": "custom", "fontSize": 14.0}
        if n == "StacShapeBorder":
            return {"type": "circleBorder"}
        return self.sample_class(self.comp_of(n), depth + 1)

    def sample_class(self, comp, depth):
        out = {}
        if comp["disc"]:
            out[comp["disc"][0]] = comp["disc"][1]
        for f in comp["fields"]:
            if depth >= 2 and f["opt"]:
                continue
            out[f["json"]] = self.sample(f["ast"], f["json"], depth)
        return out

    # ------------------------------------------------------------------ rust expr
    def rexpr(self, v, ast):
        k = ast[0]
        if k == "prim":
            t = ast[1]
            if t == "String":
                return rust_str(v)
            if t == "double":
                return rust_float(v)
            if t == "int":
                return "%d_i64" % v
            return "true" if v else "false"
        if k == "json":
            return "serde_json::json!(%s)" % json.dumps(v)
        if k == "map":
            return "json_map(serde_json::json!(%s))" % json.dumps(v)
        if k == "imap":
            inner = ", ".join("(%d_i64, %s)" % (int(key), self.rexpr(val, ast[1])) for key, val in v.items())
            return "std::collections::BTreeMap::from([%s])" % inner
        if k == "list":
            inner_rust = self.g.rust_type(ast[1], True)
            if not v:
                return "Vec::<%s>::new()" % inner_rust
            return "vec![%s]" % ", ".join(self.rexpr(x, ast[1]) for x in v)
        name = ast[1]
        if name == "StateEntry":
            return "StateEntry::new(%s, serde_json::json!(%s))" % (rust_str(v["key"]), json.dumps(v["value"]))
        if name == "StacColor":
            return rust_str(v)
        if name in self.g.enums:
            e = self.g.enums[name]
            return "%s::%s" % (self.g.rust_named(name), pascal(e["values"][e["json_values"].index(v)]))
        if name in POLY:
            base = POLY[name]
            key = "actionType" if base == "Action" else "type"
            comp = self.members[base][v[key]]
            return "%s::from(%s)" % (base, self.class_expr(v, comp))
        return self.class_expr(v, self.comp_of(name))

    def class_expr(self, v, comp):
        fields = {f["json"]: f for f in comp["fields"]}
        req = [f for f in comp["fields"] if not f["opt"]]
        args = ", ".join(self.rexpr(v[f["json"]], f["ast"]) for f in req)
        expr = "%s::new(%s)" % (comp["rust"], args)
        for key, val in v.items():
            if comp["disc"] and key == comp["disc"][0]:
                continue
            f = fields[key]
            if not f["opt"]:
                continue
            expr += "\n        .%s(%s)" % (f["ident"], self.rexpr(val, f["ast"]))
        return expr

    # ------------------------------------------------------------------ run
    def top_expr(self, comp, v):
        if comp.get("variant_of") in ("Widget", "Action", "ShapeBorder", "TextStyle"):
            return "%s::from(%s)" % (comp["variant_of"], self.class_expr(v, comp))
        return self.class_expr(v, comp)

    def run(self):
        tests = {}
        fixtures_dir = os.path.join(self.g.tests, "fixtures")
        for dart, comp in sorted(self.g.comps.items()):
            top = TOP_DIR[comp["cat"]]
            fname = snake(comp["rust"])
            fid = "%s/%s" % (top, fname)
            if "enum" in comp:
                e = comp["enum"]
                value = list(e["json_values"])
                names = ["%s::%s" % (comp["rust"], pascal(v)) for v in e["values"]]
                expr = "vec![%s]" % ", ".join(names)
            else:
                ex = self.doc_example(comp)
                value = None
                if ex is not None:
                    try:
                        value = self.conform_class(ex, comp, comp["dart"])
                    except Mismatch as m:
                        self.g.fallbacks.append((dart, str(m)))
                        value = None
                if value is None:
                    value = self.sample_class(comp, 0)
                expr = self.top_expr(comp, value)
            self.write_fixture(fixtures_dir, fid, value)
            self.g.fixture_names[dart] = fid
            tests.setdefault(top, []).append((snake(comp["rust"]), fid, expr))
        # polymorphic bases without a class of their own
        for dart, base, member in (("StacTextStyle", "TextStyle", "theme"), ("StacShapeBorder", "ShapeBorder", "roundedRectangleBorder")):
            comp = self.members[base][member]
            value = self.sample_class(comp, 0)
            top = "foundation"
            fid = "%s/%s" % (top, snake(base))
            self.write_fixture(fixtures_dir, fid, value)
            self.g.fixture_names[dart] = fid
            tests.setdefault(top, []).append((snake(base), fid, self.top_expr(comp, value)))
        for top, items in tests.items():
            out = ["// @generated by tools/stac-inventory/gen_rust.py - DO NOT EDIT\n"
                   "#![allow(unused_imports, clippy::too_many_arguments)]\n"
                   "use super::{assert_fixture, json_map};\n"
                   "use derust::sduix::stacx::common::*;\n"
                   "use derust::sduix::stacx::v1::{actions::*, foundation::*, theme::*, widgets::*};\n\n"]
            for tname, fid, expr in sorted(items):
                out.append("#[test]\nfn %s() {\n    assert_fixture!(\"%s\", %s);\n}\n\n" % (tname, fid, expr))
            self.g.write(os.path.join(self.g.tests, "generated_%s.rs" % top), "".join(out))

    def write_fixture(self, base, fid, value):
        path = os.path.join(base, fid + ".json")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(value, f, indent=2, ensure_ascii=False)
            f.write("\n")
        self.g.written.add(os.path.abspath(path))
