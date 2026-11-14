// Produces a JWE with `kid` in the protected header, to the committed node_jose public key.
// Usage: node gen_jwe_kid_fixture.mjs <dir with node_jose.public.pem>
import { importSPKI, CompactEncrypt } from 'jose';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2] ?? '.';
const plaintext = readFileSync(join(dir, 'node_jose.plaintext'), 'utf8');
const key = await importSPKI(readFileSync(join(dir, 'node_jose.public.pem'), 'utf8'), 'ECDH-ES');
const jwe = await new CompactEncrypt(new TextEncoder().encode(plaintext))
  .setProtectedHeader({ alg: 'ECDH-ES', enc: 'A256GCM', kid: 'node-kid-1' })
  .encrypt(key);
writeFileSync(join(dir, 'node_jose_kid.jwe'), jwe);
