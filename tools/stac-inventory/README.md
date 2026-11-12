# stac-inventory

Python 3 (stdlib only) tooling that turns the Stac sources into the derust `sduix::stacx::v1` model.

Pinned source: <https://github.com/StacDev/stac>, tag `v1.6.0`, commit
`b6cfe7cdb2bb4432e09d91ba23d6d72bdf721c01` (`packages/stac_core/lib`).

```bash
git clone --depth 1 --branch v1.6.0 https://github.com/StacDev/stac /tmp/stac   # never versioned
cd tools/stac-inventory
python3 generate.py /tmp/stac ../../crates/derust/src/sduix/stacx/v1/inventory.json   # 1) inventory
python3 gen_rust.py /tmp/stac ../../crates/derust                                       # 2) Rust + fixtures + tests
```

Both steps are deterministic and idempotent (running them twice leaves no diff).

* `stac_model.py` reads the Dart classes/enums and the generated `*.g.dart` files (`_$XToJson` is the
  source of truth for the JSON keys, `_$XEnumMap` for enum values).
* `stac_catalog.py` classifies every class/enum (`widget | action | foundation | theme`), assigns the
  implementation `batch` and resolves the `type` / `actionType` discriminators.
* `generate.py` writes `inventory.json`. It keeps `status`/`fixture`/`notes` already present in the file.
* `gen_rust.py` writes, marked `// @generated`, one Rust file per component (struct + builder +
  `Serialize` + `Validate`, or enum), the `Widget`/`Action`/`ShapeBorder`/`TextStyle` enums, `dsl.rs`,
  the JSON fixtures in `crates/derust/tests/sdui_stac_v1/fixtures` and the `generated_*.rs` tests, then
  runs `rustfmt`. It marks every modeled item as `supported` in the inventory. Hand written files in the
  same directories (`foundation/colors/color.rs`, `*_ext.rs`) are left alone and wired into the `mod.rs`.
* `fixtures.py` builds the fixtures: the "JSON Example" of the Dart docstring when it conforms to the
  model (unknown keys, wrong types or invalid enum values reject it) and a synthetic sample populating
  every field otherwise.

Hand written exceptions are listed at the top of `gen_rust.py` (`SKIP_CLASSES`, `FIELD_TYPE_OVERRIDES`,
`EXTRA_FIELDS`, `NESTED_ONLY_ACTIONS`) and in `custom_checks` (validation rules taken from Dart `assert`s).
