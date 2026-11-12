# sdui_reactive

Reactive SDUI screens (shared state, `bind`, typed expressions, state actions, `networkRequest`
callbacks, `navigate` with params/result, list templates) built only with the typed API of
`derust::sduix::stacx::v1::prelude`. Needs the derust-stac fork on the client (branch
`estado_compartilhado_reativo`); the official Stac does not understand this vocabulary.

```bash
cargo run
curl -i localhost:3000/screens/form        # 200, content-type: application/json
curl -i localhost:3000/screens/list
curl -i localhost:3000/screens/network
curl -i localhost:3000/screens/navigation
curl -i localhost:3000/screens/invalid     # 500, empty body (the DerustError message names $.children[0].onPressed...)
```

The screens live in `src/screen.rs`, also compiled by
`crates/derust/tests/sdui_stac_v1/example_reactive_test.rs`, which compares them with
`fixtures_reactive/screens/example_*.json` (parsed by the fork in `tools/stac-compat/v1-reactive`).
