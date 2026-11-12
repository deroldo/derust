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

Hand written exceptions are listed at the top of `gen_rust.py` (`SKIP_CLASSES`, `SKIP_FIELDS`,
`FIELD_TYPE_OVERRIDES`, `EXTRA_FIELDS`, `EXTRA_ACTIONS`, `NESTED_ONLY_ACTIONS`) and in `custom_checks`
(validation rules taken from Dart `assert`s).

## Reactive layer (derust-stac fork vocabulary)

On top of the official model the generator adds the reactive vocabulary of the fork (types live,
hand written, in `v1/state`); the official fixtures never contain it (fields marked `fork` are skipped by
`fixtures.py`, so `fixtures/` and the `v1.6.0` harness are unchanged):

* **Envelope.** Every widget gets `envelope: Envelope` (serialized right after `type`, through
  `Envelope::serialize_into`) and the setters `visible`, `enabled`, `bind`, `state`, `global_state`,
  `on_init`. `SKIP_FIELDS` lists the native `visible`/`enabled` fields that would collide
  (`Visibility`, `SliverVisibility`, `BackdropFilter`, `DropdownMenu`, `ListTile`, `Radio`, `TextField`,
  `TextFormField`): the field is removed and the setter of the same name writes the envelope (same JSON
  key, `Reactive<bool>`; a plain `bool` still works). The fixtures still carry those keys.
* **`condition`.** Every registered action (not `NESTED_ONLY_ACTIONS`) gets `condition: Option<Reactive<bool>>`.
* **`Reactive<T>`.** Every leaf property (anything but `Widget`, `Action`, `Vec<Widget>`, `Vec<Action>`
  and the `String` properties, which stay `Dyn<String>`) becomes `Option<Reactive<T>>` (boxed fields:
  `Reactive<Box<T>>`) and its setter takes `impl IntoReactive<T>`. Every generated type gets `impl Plain`;
  the variants of `TextStyle` / `ShapeBorder` get an `IntoReactive<Base>` impl so they keep being accepted.
* **Hooks (infrastructure for the fork, unused until a task needs them).**
  `EXTRA_ACTIONS = [(Variant, "actionType", "rust::path::Type")]` links a hand written action to the
  `Action` enum (variant, `Serialize` / `Validate` / `type_name` arms and `From`).
  `FIELD_TYPE_OVERRIDES` / `EXTRA_FIELDS` accept the pseudo types `Data`, `Actions`, `StringList`,
  `ItemTemplate` (hand written Rust types of `v1::state`, never wrapped in `Reactive`) and an explicit
  `Reactive<...>` (e.g. `Reactive<List<Data>>?`). Such fields are optional and are never part of the
  official fixtures.

The source is always the official Stac (`StacDev/stac`). The project's fork
[`deroldo/derust-stac`](https://github.com/deroldo/derust-stac) holds extra Dart parsers used with derust;
those are not Stac components, so they are not part of the inventory, the generated model or the
fixtures (the server emits them through `CustomWidget` / `CustomAction`).
