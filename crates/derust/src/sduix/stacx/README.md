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

| derust | feature | vocabulary | runtime required on the client |
|---|---|---|---|
| `0.7.0` | `sdui_stac_1` | reactive state (see [Reactive state](#reactive-state)) | fork [`deroldo/derust-stac`](https://github.com/deroldo/derust-stac), branch `estado_compartilhado_reativo`, **no pinned version** (contract read at commit `92457f25`) |

The reactive vocabulary is **not** part of the official Stac `1.6.0`: a frontend running the official Stac
ignores it or fails on it. The matrix and the `CHANGELOG.md` are the only protection. The reactive part
is checked by `tools/stac-compat/v1-reactive` against the fork (CI job `stac-compat-v1-reactive`,
`continue-on-error` until a pin exists; the pin is the single `ref` in its `pubspec.yaml`).

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
* `Widget` and `Action` are enums of the Stac components (`type` / `actionType` discriminators) plus a
  `Custom` variant for the application's own parsers (see [Extending](#extending)); every struct converts
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

## Reactive state

Needs the `derust-stac` fork on the client (see the matrix). The server only *describes* state and
reactions; the client owns the values. Everything below is built with Rust types: no JSON and no
expression text is written by hand.

* **State blocks.** Any widget can carry `state` (screen keys, `Data` initial values), `globalState`
  (`GlobalEntry`: a plain value or `initial` + `persist` / `secure`) and `onInit` (one action or a list).
  A widget with its own `state` opens a scope that **replaces** the outer one.
* **Envelope.** Every widget has `visible`, `enabled` (reactive booleans), `bind`, `state`,
  `global_state`, `on_init`. `bind(..)` takes `impl Into<BindKey>`: `"name"` (screen key),
  `"global.name"` or a `BindKey`. `Slider::bound("volume")` builds a slider with `bind` and no `value`
  (`Slider.value` is optional: `value` **or** `bind`, never both).
* **Bindings and expressions.** Every leaf property is `Option<Reactive<T>>`; setters take a plain value,
  a `StatePath`, an `Expr` or a `Binding` (serialized as `{"$state": ".."}` / `{"$expr": ".."}`).
  `Expr` is a closed enum (literals, references, `.field`, `[index]`, unary, binary, ternary, grouping and
  the functions `length`, `trim`, `contains` with 2 or 3 arguments) with operator sugar
  (`Expr::length(Expr::trim(name)).lt(3)`, `Expr::cond(..)`); an unknown function or a wrong arity cannot be written.
  The printer adds the minimum parentheses and escapes text. `StatePath` has no operators.
* **Actions.** `setState`, `incrementState`, `decrementState`, `toggleState`, `resetState`,
  `appendState`, `removeFromState` (`at_index` / `by_value`), `toggleItem`, `cancelRequest` and `none`.
  Every action accepts `condition`.
* **`networkRequest`.** `onStart`, `onSuccess`, `onError`, `onFinally` (one action or a list, order kept),
  `requestKey`, `bodyFromState`. `response` exists in `onSuccess` / `onError` / `onFinally`, not in `onStart`.
* **`navigate`.** `params` (read as `params.*` in the destination), `result`, `onResult`, `onCancel`.
  `onResult` runs when the route closes with a result, `onCancel` when it closes without one; **both may
  be set**. `result` is only valid with `pop`, `pushReplacement` and `pushReplacementNamed` (no
  `navigationStyle` counts as `push`).
* **Lists.** `ListView`, `GridView`, `PageView`, `SliverList`, `SliverGrid` take
  `template(ItemTemplate::new(items).item_as("todo").index_as("i"), |vars| widget)`. `vars.item()` and
  `vars.index()` carry the effective names (`item` / `index` by default). `item` and `index` are
  read-only local variables.
* **Per item state.** Use screen (or global) state indexed by item identity: a key holding a list of ids
  (`selected`), `contains(selected, item.id)` for membership, `toggleItem` / `appendState` /
  `removeFromState` to edit it and `length(selected)` for aggregates. `bind` does not work per item
  (it writes a whole key). **Limit:** editing a field of an item in place (`todos[i].done = ..`) has no
  action in the contract and is not covered.

Full example: [`examples/sdui_reactive`](../../../../../examples/sdui_reactive).

### When validation fails

`SduiResponse::try_new` runs `Validate` plus a check of the serialized JSON and answers 500
(`DerustError`) when there is an **error**; the message carries the JSON path (`$.children[2]...`) and only
key/property names, never values. Errors:

* an identifier that is empty, malformed, `true`, `false` or `null`; a non finite number or an integer
  beyond `i64`; a dynamic index that is a division or has a fractional part;
* a data map with exactly one `$state` / `$expr` text key (the contract has no escape);
* an empty `bind` key, a state key that is reserved (`global params result response event`), or a state
  key declared with `persist` / `secure` (only `globalState` has them);
* `secure: true` together with `persist: false`; `bodyFromState` pointing to a secure global key;
* `bind` on a widget that is not a checkBox, switch, slider, radioGroup, textField, textFormField or
  dropdownMenu (including `radio`), or `bind` together with the value property it overrides;
* a write (`setState`, `incrementState`, ..., `toggleItem`) to an undeclared screen key when the scope is
  known, to a reserved name, or to `item` / `index` / the template names (read only);
* `response`, `event`, `result`, `item` / `index` used outside the place where they exist;
* `navigate` `result` with a style other than `pop`, `pushReplacement`, `pushReplacementNamed`; a
  `removeFromState` without target;
* `state` / `globalState` that is not an object; a screen nested too deep.

Only a **warning** (logged with `tracing::warn!`, never blocking): reading an undeclared screen key, a
template variable that shadows one of an outer template, and a template root that declares its own `state`
(it hides the keys of the screen, including aggregates). The root of a response, `navigate.widgetJson` and
overlays (`showDialog`, `showModalBottomSheet`, `showSnackBar`) have an opaque scope (nothing is
reported there), and `params` is never reported.

### What the server cannot verify

These are runtime behaviours of the client; derust does not check them:

* comparing mixed types (`'a' < 1`), division by zero, `!` on a non boolean, and any result that depends
  on the value's type: the runtime yields `null` or `false`;
* `+` concatenation: with `null`, a list or an object the result is `null`; an integer-valued number
  prints without `.0`;
* a dynamic index must be an integer (`/` never gives a valid index);
* short circuit: `&&`, `||` and `?:` are evaluated by the runtime and derust never reorders operands;
* `global.key` and `global['key']` are the same key; derust always prints `global.key`;
* `enabled` is only applied by a closed list of widgets; on others it has no effect;
* `secure` values never appear in derust logs or messages (the client masks them by text substitution).

### Fork v1 limitations (documentation only)

From the "v1 limitations" of the contract; derust does not work around them:

* `params` with named routes (`pushNamed*`) is delivered only in `arguments["__stacParams"]`;
* list item identity is the index (insert/remove in the middle shifts local widget state);
* a nested `state` does not chain with the outer one;
* `secure` masking is text substitution (a short secret hides the same text elsewhere);
* the `separator` of a list is outside the item scope (no `item` / `index`);
* the slider `onChangeStart` / `onChangeEnd` have no `event.value` (read the bound key);
* the contract text says `onResult` and `onCancel` are mutually exclusive; the project decided that they
  coexist (the contract needs a fix in the fork).

## Extending

### Reusable compositions

A screen template (standard header/footer, a design system card...) made of Stac widgets is plain Rust:
implement `From<YourType> for Widget` and use it anywhere a widget is accepted. The JSON contains only
standard Stac widgets, so the Flutter app needs nothing.

```rust
use derust::sduix::stacx::v1::prelude::*;

pub struct StandardScreen {
    title: String,
    body: Widget,
}

impl StandardScreen {
    pub fn new(title: impl Into<String>, body: impl Into<Widget>) -> Self {
        Self { title: title.into(), body: body.into() }
    }
}

impl From<StandardScreen> for Widget {
    fn from(screen: StandardScreen) -> Self {
        Widget::from(
            scaffold()
                .app_bar(app_bar().title(text(screen.title)))
                .body(screen.body),
        )
    }
}

let screen = Widget::from(StandardScreen::new("Home", text("hello")));
assert_eq!(serde_json::to_value(&screen).unwrap()["appBar"]["type"], "appBar");
```

### Custom widgets and actions

When the Flutter app registers its own parsers (`Stac.initialize(parsers: [..], actionParsers: [..])`),
the server emits them through `CustomWidget` / `CustomAction`. The struct serializes its fields (as a
JSON object, without the discriminator) and validates itself, including any standard widget it holds;
derust adds `type` / `actionType`. A field clashing with the discriminator, or a value that does not
serialize as an object, is a validation issue (so `SduiResponse::try_new` fails with a 500).

```rust
use derust::sduix::stacx::v1::prelude::*;
use serde::Serialize;

/// Rendered by a `stateText` parser registered in the app.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StateText {
    state_key: String,
}

impl Validate for StateText {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if self.state_key.is_empty() {
            issues.push(ValidationIssue::new(&path.key("stateKey"), "must not be empty"));
        }
    }
}

impl CustomWidget for StateText {
    const TYPE_NAME: &'static str = "stateText";
}

/// Executed by a `setState` action parser registered in the app.
#[derive(Debug, Serialize)]
struct SetState {
    key: String,
    value: String,
}

impl Validate for SetState {
    fn validate_at(&self, _path: &Path<'_>, _issues: &mut Vec<ValidationIssue>) {}
}

impl CustomAction for SetState {
    const ACTION_TYPE: &'static str = "setState";
}

let screen = Widget::from(column().children([
    Widget::from(StateText { state_key: "user.name".into() }),
    Widget::from(
        text_button()
            .on_pressed(SetState { key: "user.name".into(), value: "Ada".into() })
            .child(text("Set name")),
    ),
]));
let json = serde_json::to_value(&screen).unwrap();
assert_eq!(json["children"][0], serde_json::json!({"type": "stateText", "stateKey": "user.name"}));
assert_eq!(json["children"][1]["onPressed"]["actionType"], "setState");
assert!(screen.validate().is_empty());
```

Custom nodes are not part of Stac, so they are outside `inventory.json`, the fixtures and the Flutter
compatibility harness: their contract with the app's parser is the application's responsibility.

### The `derust-stac` fork

[`deroldo/derust-stac`](https://github.com/deroldo/derust-stac) is the project's fork of Stac. It is
where Dart parsers that derust relies on but Stac does not ship (for instance a widget that rebuilds
when client state changes) are maintained, so they can be reused by every app instead of being written
in each one. The typed model in this module is still generated from, and checked against, the official
Stac (see the [compatibility matrix](#compatibility-matrix)); nodes provided by the fork reach the
server through `CustomWidget` / `CustomAction`.

## Coverage (`v1/inventory.json`)

Every class and enum of `packages/stac_core` at `v1.6.0` is listed with a `status`:

* `supported` - modeled (struct/enum, builder, fixture and test);
* `partial` - modeled with documented limitations (`notes`);
* `unsupported` - Dart-side helpers with no JSON representation (`notes` explains each one).

Current inventory: **259 supported, 0 partial, 11 unsupported** (98 widgets, 15 action items,
123 foundation and 34 theme items in total).

The reactive vocabulary has its own inventory, `v1/inventory_state.json` (`source: "fork"`, written by
hand): **149 supported, 7 unsupported** (envelope properties, actions, action/widget fields, operators,
functions, literals, binding forms, variable roots, contract examples). The 7 unsupported items are
limits of the typed model or of the contract: `$state` with an operation, binding-shaped literal data,
unknown function or wrong arity, `bind` on `radio`, binding a single element of a list property,
in-place edit of item fields, and a `separator` reading `item` / `index`. Every `supported` item has a
fixture (`tests/sdui_stac_v1/fixtures_reactive`) and a test.

Things to know:

* Icon names are plain strings (the Dart `StacIcons` constants are not mirrored).
* `StacColor` is `Color` (`Color::hex("#FF0000")`, `.with_opacity(0.5)`).
* `Method`, `NavigationStyle` and the other enums keep the exact Dart value names in the JSON.
* Large screens count against the server timeout (`SERVER_TIMEOUT_IN_MILLIS`) and are gzip-compressed by
  the derust compression layer.

## Regenerating

Almost all of `v1` is generated from the Stac sources by `tools/stac-inventory` (see its README);
do not edit `// @generated` files by hand.
