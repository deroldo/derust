// Generates the independent-implementation JWE fixture (run once; output is committed).
// Usage: npm install jose && node gen_jwe_fixture.mjs <output-dir>
import { generateKeyPair, exportPKCS8, exportSPKI, CompactEncrypt } from 'jose';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const out = process.argv[2] ?? '.';
const plaintext = 'derust cryptox interop: olá, mundo ✓';
const { publicKey, privateKey } = await generateKeyPair('ECDH-ES', { crv: 'P-256', extractable: true });
const jwe = await new CompactEncrypt(new TextEncoder().encode(plaintext))
  .setProtectedHeader({ alg: 'ECDH-ES', enc: 'A256GCM' })
  .encrypt(publicKey);
writeFileSync(join(out, 'node_jose.private.pem'), await exportPKCS8(privateKey));
writeFileSync(join(out, 'node_jose.public.pem'), await exportSPKI(publicKey));
writeFileSync(join(out, 'node_jose.jwe'), jwe);
writeFileSync(join(out, 'node_jose.plaintext'), plaintext);
