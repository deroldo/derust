# stac-compat v1

Flutter test project that parses every JSON fixture written by the derust test-suite
(`crates/derust/tests/sdui_stac_v1/fixtures`) with the **real Stac `v1.6.0`**
(`package:stac` pinned by git tag in `pubspec.yaml`, together with `stac_core`,
`stac_framework` and `stac_logger` of the same tag).

```bash
cd tools/stac-compat/v1
flutter pub get
flutter test
```

Entry point: after `Stac.initialize()` the parsers are looked up in `StacRegistry.instance`
and `StacParser.getModel` / `StacActionParser.getModel` is called. That runs the strict
`json_serializable` `fromJson` of the model (no device, no rendering). `Stac.fromJson` is not
used because it swallows parse errors and would let invalid fixtures pass.

* widgets/actions: registry lookup of `type` / `actionType`; an unregistered root type fails, and
  nested widgets/actions are checked recursively;
* other models: `Stac<Name>.fromJson` (map generated into `test/fixture_models.g.dart`);
* enums: the fixture must list exactly the values of the Dart enum;
* `test/negative/*.json`: invalid JSON that must be rejected (proves the harness detects divergence);
* `allowlist.txt`: `fixture-id: reason` for fixtures Stac itself cannot parse (keep it empty
  unless the gap is in Stac).

One directory per Stac major (`v2` in the future) and one CI job per major
(`.github/workflows/ci.yml`).

The harness pins the official Stac on purpose: it proves compatibility with what apps get from
pub.dev / `StacDev/stac`. Parsers from the project's fork
[`deroldo/derust-stac`](https://github.com/deroldo/derust-stac) are extensions outside the Stac vocabulary
and are not checked here.
