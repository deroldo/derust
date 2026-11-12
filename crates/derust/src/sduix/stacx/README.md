# sduix::stacx - server-driven UI for [Stac](https://github.com/StacDev/stac)

Build Stac screens in Rust with 100% typed widgets, actions, styles and themes, validate them and
serve them as JSON. The backend only *describes* screens: it never reads them back, and the state
referenced by `{{key}}` lives in the Stac client (persisting it is the application's job).

## Enabling

```toml
# only the types (no HTTP server pulled in)
derust = { version = "0.7", default-features = false, features = ["sdui_stac_1"] }

# types + `SduiResponse` for Axum handlers
derust = { version = "0.7", features = ["sdui_stac_1"] }
```

There is one feature per Stac **major**: `sdui_stac_1` -> `derust::sduix::stacx::v1`. A future
`sdui_stac_2` will add `v2` next to it without changing anything existing.

> Enable **only the one** Stac feature your frontend uses. Enabling several is supported, but each
> major adds hundreds of types and increases compile time.

## Compatibility matrix

| derust | feature | module | Stac (tag / commit) |
|---|---|---|---|
| `0.7.0` | `sdui_stac_1` | `sduix::stacx::v1` | `1.6.0` (`v1.6.0` / `b6cfe7cdb2bb4432e09d91ba23d6d72bdf721c01`) |

In code: `derust::sduix::stacx::versions()` (enabled majors) and
`derust::sduix::stacx::v1::{STAC_VERSION, STAC_TAG_COMMIT}`. Deprecations and removals are announced in
the `CHANGELOG.md`, with no time limit on the window.

The model is checked against the real Stac: every component has a JSON fixture, a Rust test that builds
it with the typed API, and a Flutter test (`tools/stac-compat/v1`, CI job `stac-compat-v1`) that parses
the same fixture with Stac `v1.6.0`. If your frontend runs a different Stac version than this feature,
that matrix is the only protection.

## Minimal example

```rust,no_run
use derust::commonx::{DerustError, DerustTags};
use derust::httpx::sdui::SduiResponse;
use derust::sduix::stacx::v1::prelude::*;
use derust::StatusCode;

async fn home() -> Result<SduiResponse<Widget>, DerustError> {
    let screen = Widget::from(
        scaffold()
            .app_bar(app_bar().title(text("Home")))
            .body(column().children([
                Widget::from(text(StateRef::new("user.name"))),
                Widget::from(
                    elevated_button()
                        .on_pressed(navigate().route_name("/details"))
                        .child(text("Details")),
                ),
            ])),
    );
    SduiResponse::try_new(StatusCode::OK, screen, DerustTags::default())
}
```

A complete application lives in [`examples/sdui`](../../../../../examples/sdui).

## API

* Types: one struct per Stac component (`Column`, `NavigateAction`, `EdgeInsets`, `StacTheme`...), with
  `Type::new(<required fields>)` and chained setters (`.padding(..)`). Unset optional fields are omitted
  from the JSON (the Stac client applies its own defaults).
* `Widget` and `Action` are closed enums (`type` / `actionType` discriminators); every struct converts
  with `Widget::from(..)` / `Action::from(..)`, and setters that take a widget/action accept the struct
  directly (`.on_pressed(navigate())`).
* `v1::dsl` (reexported by `v1::prelude`): shortcuts such as `column()`, `text("..")`, `navigate()`.
* Strings accept a literal or a state reference: `text("Hello")` or `text(StateRef::new("user.name"))`
  (serialized as `{{user.name}}`, replaced by the client).
* `Validate::validate()` returns the `ValidationIssue`s of a tree (non finite numbers, empty state
  references and the `assert`s of the Dart models). Stac itself imposes no other structural
  constraints on children at parse time.
* `httpx::sdui::SduiResponse<T>`: `SduiResponse::try_new(status, screen, tags)` validates and serializes
  once; failures become a `DerustError` (500) instead of a broken response. `Content-Type:
  application/json`, `x-trace-id` tag and `with_headers` behave like `JsonResponse`.

## Coverage (`v1/inventory.json`)

Every class and enum of `packages/stac_core` at `v1.6.0` is listed with a `status`:

* `supported` - modeled (struct/enum, builder, fixture and test);
* `partial` - modeled with documented limitations (`notes`);
* `unsupported` - Dart-side helpers with no JSON representation (`notes` explains each one).

Current inventory: **259 supported, 0 partial, 11 unsupported** (98 widgets, 15 action items,
123 foundation and 34 theme items in total).

Things to know:

* Icon names are plain strings (the Dart `StacIcons` constants are not mirrored).
* `StacColor` is `Color` (`Color::hex("#FF0000")`, `.with_opacity(0.5)`).
* `Method`, `NavigationStyle` and the other enums keep the exact Dart value names in the JSON.
* Large screens count against the server timeout (`SERVER_TIMEOUT_IN_MILLIS`) and are gzip-compressed by
  the derust compression layer.

## Regenerating

Almost all of `v1` is generated from the Stac sources by `tools/stac-inventory` (see its README);
do not edit `// @generated` files by hand.
