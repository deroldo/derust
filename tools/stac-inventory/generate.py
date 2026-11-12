#!/usr/bin/env python3
"""Generates `inventory.json` (coverage of Stac `stac_core`) for a given Stac checkout.

Usage:
    python3 tools/stac-inventory/generate.py <stac-checkout> <output.json>

The generator is idempotent and keeps manual curation (`status`, `fixture`, `notes`)
already present in an existing output file.
"""

import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stac_catalog  # noqa: E402


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    checkout, out_path = sys.argv[1], sys.argv[2]
    lib_root = os.path.join(checkout, "packages", "stac_core", "lib")
    _, items = stac_catalog.build(lib_root)

    previous = {}
    if os.path.exists(out_path):
        with open(out_path, encoding="utf-8") as f:
            previous = {i["name"]: i for i in json.load(f)["items"]}

    for it in items:
        old = previous.get(it["name"], {})
        it["status"] = old.get("status", "unsupported")
        it["fixture"] = old.get("fixture")
        if "notes" in old:
            it["notes"] = old["notes"]
        elif it["name"] in stac_catalog.UNSUPPORTED_NOTES:
            it["status"] = "unsupported"
            it["notes"] = stac_catalog.UNSUPPORTED_NOTES[it["name"]]

    tag = commit = None
    try:
        commit = subprocess.check_output(
            ["git", "-C", checkout, "rev-parse", "HEAD"], text=True).strip()
        tag = subprocess.check_output(
            ["git", "-C", checkout, "describe", "--tags", "--exact-match"], text=True).strip()
    except Exception:  # noqa: BLE001
        pass
    doc = {"stac_tag": tag, "stac_commit": commit, "items": items}
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(doc, f, indent=2, sort_keys=False)
        f.write("\n")
    by = {}
    for it in items:
        by[it["category"]] = by.get(it["category"], 0) + 1
    print("items:", len(items), by)


if __name__ == "__main__":
    main()
