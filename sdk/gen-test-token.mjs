import * as jose from 'jose';
import { readFileSync, existsSync } from 'fs';
import { execSync } from 'child_process';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');
const keyPath = resolve(projectRoot, 'deploy', 'dev-certs', 'jwt', 'private.key');

// Auto-generate key if missing
if (!existsSync(keyPath)) {
  console.error("JWT dev key not found, generating...");
  execSync(resolve(projectRoot, 'scripts', 'gen-dev-keys.sh'), { stdio: 'inherit' });
}

const JWKS = jose.createRemoteJWKSet(new URL('http://localhost:8080/.well-known/jwks.json'));
const KEY = await jose.importPKCS8(
  readFileSync(keyPath, 'utf-8'),
  'RS256'
);

const DEMO_TENANT_ID = 'cc9c7f62-c753-40b2-bb6e-b9b7b65d3133';

const token = await new jose.SignJWT({
  sub: 'test-user-123',
  email: 'testuser@demo.com',
  tenant_id: DEMO_TENANT_ID,
  roles: ['admin', 'editor'],
  permissions: ['user:read', 'user:write', 'post:read'],
})
  .setProtectedHeader({ alg: 'RS256', typ: 'JWT', kid: 'auth9-current' })
  .setIssuer('http://localhost:8080')
  .setAudience('auth9-portal')
  .setIssuedAt()
  .setExpirationTime('1h')
  .sign(KEY);

console.log(token);
