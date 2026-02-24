import * as jose from 'jose';
import { readFileSync } from 'fs';

const JWKS = jose.createRemoteJWKSet(new URL('http://localhost:8080/.well-known/jwks.json'));
const KEY = await jose.importPKCS8(
  readFileSync('/Volumes/Yotta/auth9/.claude/skills/tools/jwt_private_clean.key', 'utf-8'),
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
