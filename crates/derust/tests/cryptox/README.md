# cryptox JWE interoperability fixtures

`node_jose.*` were produced once by an implementation independent of derust and josekit:
Node `jose` 6.2.17 (Node v26), ECDH-ES + A256GCM, EC P-256. The private key is a throwaway
test key; never use it for anything else.

Regenerate (only needed if the fixture must change):

```bash
mkdir /tmp/jose-tools && cd /tmp/jose-tools && npm init -y && npm install jose
cp <repo>/crates/derust/tests/cryptox/*.mjs .
node gen_jwe_fixture.mjs <repo>/crates/derust/tests/cryptox
```

Forward direction (independent -> derust): unit test `cryptox::jwe::test::opens_jwe_produced_by_node_jose`
opens `node_jose.jwe` with `JweOpener`.

Reverse direction (derust -> independent):

```bash
CRYPTOX_JWE_OUT=/tmp/seal.jwe cargo nextest run --features cryptox writes_seal_for_node_jose
node open_jwe.mjs crates/derust/tests/cryptox/node_jose.private.pem /tmp/seal.jwe
# prints the plaintext (needs `npm install jose` next to open_jwe.mjs)
```

## JWE with `kid`

`node_jose_kid.jwe` was produced by Node `jose` for the same key, with `kid: "node-kid-1"` in the protected
header (`node gen_jwe_kid_fixture.mjs <this dir>`). Unit test
`cryptox::jwe::test::opens_jwe_with_kid_produced_by_node_jose` opens it with `JweOpener` and `JweOpenerSet`.

Reverse direction (derust `JweSealer::with_kid` -> Node `jose`):

```bash
CRYPTOX_JWE_KID_OUT=/tmp/seal_kid.jwe cargo nextest run --features cryptox writes_seal_with_kid
node open_jwe.mjs crates/derust/tests/cryptox/node_jose.private.pem /tmp/seal_kid.jwe
# prints the plaintext; stderr shows the header including the kid
```
