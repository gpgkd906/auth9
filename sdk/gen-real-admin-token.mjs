#!/usr/bin/env node
import { createMockToken } from "./packages/node/dist/testing.js";
import { execSync } from "child_process";

// Get real user ID from database
const userId = execSync(
  'mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = \'admin@auth9.local\' LIMIT 1;"'
).toString().trim();

const token = createMockToken({
  sub: userId,
  email: "admin@auth9.local",
  name: "Admin User",
  aud: "auth9",
});

process.stdout.write(token);
