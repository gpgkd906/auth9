#!/usr/bin/env node
const jwt = require('jsonwebtoken');
const fs = require('fs');
const path = require('path');

const keyPath = path.resolve(__dirname, 'jwt_private_clean.key');
const privateKey = fs.readFileSync(keyPath, 'utf8');

const now = Math.floor(Date.now() / 1000);
const issuer = "http://localhost:8080";
const ttl = 3600;

const SERVICE_ID = process.env.SERVICE_ID || "f7bf6609-9e6a-48cf-864f-7f2f091eed10";
const TENANT_ID = process.env.TENANT_ID || "8bbb1966-b86e-4a1a-a2c1-83a94f7ee62f";

const payload = {
    sub: SERVICE_ID,
    email: `service+${SERVICE_ID}@auth9.local`,
    iss: issuer,
    aud: "auth9-service",
    tenant_id: TENANT_ID,
    iat: now,
    exp: now + ttl
};

const token = jwt.sign(payload, privateKey, { algorithm: 'RS256', keyid: 'auth9-current' });
process.stdout.write(token);
