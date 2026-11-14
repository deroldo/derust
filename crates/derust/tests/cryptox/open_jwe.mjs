// Opens a compact JWE (ECDH-ES / A256GCM) with an independent implementation.
// Usage: node open_jwe.mjs <private.pem> <jwe-file-or-string>
import { compactDecrypt, importPKCS8 } from 'jose';
import { readFileSync, existsSync } from 'node:fs';

const [pemPath, jweArg] = process.argv.slice(2);
const key = await importPKCS8(readFileSync(pemPath, 'utf8'), 'ECDH-ES');
const jwe = existsSync(jweArg) ? readFileSync(jweArg, 'utf8').trim() : jweArg;
const { plaintext, protectedHeader } = await compactDecrypt(jwe, key);
process.stdout.write(new TextDecoder().decode(plaintext));
process.stderr.write('\nheader: ' + JSON.stringify(protectedHeader) + '\n');
