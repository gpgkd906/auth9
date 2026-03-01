const jwt = require('jsonwebtoken');
const fs = require('fs');
const crypto = require('crypto');

const keyPath = './deploy/dev-certs/jwt/private.key';
const privateKey = fs.readFileSync(keyPath, 'utf8');

const userId = '3aedee2d-8f25-44de-93bb-1ef5d58e84c3'; // admin@auth9.local
const tenantId = '3427371a-b594-4d47-9c67-d876cab0522b'; // demo tenant
const serviceId = '5eadeb71-039e-45b1-9184-657a365b5794'; // Auth9 Admin Portal
const now = Math.floor(Date.now() / 1000);

const payload = {
  sub: userId,
  sid: 'test-session-' + now,
  email: 'admin@auth9.local',
  iss: 'http://localhost:8080',
  aud: serviceId,
  token_type: 'access',
  tenant_id: tenantId,
  roles: ['admin'],
  permissions: ['Full Admin Access'],
  iat: now,
  exp: now + 3600
};

const token = jwt.sign(payload, privateKey, { algorithm: 'RS256', keyid: 'auth9-current' });
console.log(token);
