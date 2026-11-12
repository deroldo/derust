# stac-compat v1-reactive

Flutter test project that checks the reactive-state vocabulary written by the derust test-suite
(`crates/derust/tests/sdui_stac_v1/fixtures_reactive`) against the **derust-stac fork**
(<https://github.com/deroldo/derust-stac>, branch `estado_compartilhado_reativo`). The official
harness (`../v1`) keeps pinning Stac `v1.6.0` and is untouched.

## Pin

The only place that pins the fork is the `ref` in `pubspec.yaml` (repeated in
`dependency_overrides`, because `stac`, `stac_core`, `stac_framework` and `stac_logger` must
come from the same ref). It is a branch today, so there is **no pin**: the CI job is allowed to
fail (`continue-on-error`) until the project picks a tag or commit.

To test against a local clone without editing `pubspec.yaml`, create `pubspec_overrides.yaml`
(git-ignored) with `dependency_overrides` of `stac`, `stac_core`, `stac_framework` and
`stac_logger` pointing to `path:` of the clone (`packages/<name>`).

```bash
cd tools/stac-compat/v1-reactive
flutter pub get
flutter test
```

## What runs

* `test/expression_dump_test.dart`: for every `{source, sexp}` pair of
  `fixtures_reactive/expressions/*.json` (generated from typed `Expr` values by
  `reactive_expressions.rs`), parses `source` with the real expression parser of the fork
  (`parseExpression` of `package:stac/src/state/expression/parser.dart`), dumps the AST in the
  format documented on `Expr::sexp()` and compares. This catches any divergence between the
  precedence/associativity/escaping of the Rust printer and the parser of the fork.
* `test/smoke_test.dart`: the fork is wired in and recognises a binding in a widget.

Planned (T9b): `compat_reactive_test.dart` walking `fixtures_reactive/**` and the official
`fixtures/**`. Note that the strict model parsers do not accept a raw binding (`data` is cast to
`String`): the runtime resolves bindings first (`StacBinding.resolve`), so that test has to do the
same before calling `getModel`.

## Regenerating the expression fixtures

```bash
UPDATE_REACTIVE_FIXTURES=1 cargo nextest run --package derust --features sdui_stac_1 expression
```

Review the diff: the Dart test is the independent judge of the new lines.
