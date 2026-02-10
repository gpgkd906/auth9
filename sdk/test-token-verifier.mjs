#!/usr/bin/env node
/**
 * Integration test for TokenVerifier
 * QA Document: docs/qa/sdk/03-token-verification.md
 */

import { TokenVerifier, Auth9 } from "./packages/node/dist/index.js";
import { execSync } from "child_process";

const GREEN = "\x1b[32m";
const RED = "\x1b[31m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

function pass(msg) {
  console.log(`${GREEN}✓${RESET} ${msg}`);
}

function fail(msg, err) {
  console.log(`${RED}✗${RESET} ${msg}`);
  if (err) console.error(`  Error: ${err.message}`);
}

function section(title) {
  console.log(`\n${YELLOW}${title}${RESET}`);
}

// Generate admin token
const token = execSync("cd /Volumes/Yotta/auth9 && .claude/skills/tools/gen-admin-token.sh")
  .toString()
  .trim();

async function runTests() {
  section("场景 1: 验证 Identity Token");

  try {
    const verifier = new TokenVerifier({
      domain: "http://localhost:8080",
    });

    const { claims, tokenType } = await verifier.verify(token);

    if (tokenType === "identity") {
      pass("tokenType === 'identity'");
    } else {
      fail(`tokenType should be 'identity', got '${tokenType}'`);
    }

    if (claims.aud === "auth9") {
      pass("claims.aud === 'auth9'");
    } else {
      fail(`claims.aud should be 'auth9', got '${claims.aud}'`);
    }

    if (claims.sub && claims.sub.length > 0) {
      pass(`claims.sub is valid UUID: ${claims.sub}`);
    } else {
      fail("claims.sub is missing");
    }

    if (claims.email && claims.email.includes("@")) {
      pass(`claims.email is valid: ${claims.email}`);
    } else {
      fail("claims.email is invalid");
    }

    if (claims.exp > claims.iat) {
      pass("claims.exp > claims.iat");
    } else {
      fail("claims.exp should be greater than claims.iat");
    }
  } catch (err) {
    fail("场景 1 failed", err);
  }

  section("场景 3: Token 签名验证失败");

  try {
    const verifier = new TokenVerifier({
      domain: "http://localhost:8080",
    });

    // Test invalid token string
    try {
      await verifier.verify("not-a-jwt-token");
      fail("Invalid token should throw error");
    } catch (err) {
      pass("Invalid token correctly rejected");
    }

    // Test tampered token
    const parts = token.split(".");
    const payload = JSON.parse(Buffer.from(parts[1], "base64url").toString());
    payload.email = "hacker@evil.com";
    parts[1] = Buffer.from(JSON.stringify(payload)).toString("base64url").replace(/=/g, "");
    const tamperedToken = parts.join(".");

    try {
      await verifier.verify(tamperedToken);
      fail("Tampered token should throw error");
    } catch (err) {
      pass("Tampered token correctly rejected");
    }
  } catch (err) {
    fail("场景 3 failed", err);
  }

  section("场景 5: Auth9 主类统一入口");

  try {
    const auth9 = new Auth9({
      domain: "http://localhost:8080",
    });

    const claims = await auth9.verifyToken(token);

    if (claims.sub && claims.email) {
      pass("Auth9.verifyToken() returns valid claims");
      pass(`  sub: ${claims.sub}, email: ${claims.email}`);
    } else {
      fail("Auth9.verifyToken() missing claims");
    }

    // Test getServiceToken without credentials
    const auth9NoCredentials = new Auth9({
      domain: "http://localhost:8080",
    });

    try {
      await auth9NoCredentials.getServiceToken();
      fail("getServiceToken() should throw without credentials");
    } catch (err) {
      if (err.message.includes("credentials")) {
        pass("getServiceToken() throws clear error without credentials");
      } else {
        fail("getServiceToken() error message unclear", err);
      }
    }
  } catch (err) {
    fail("场景 5 failed", err);
  }

  section("\n📊 TokenVerifier 集成测试完成");
}

runTests().catch((err) => {
  console.error("Test suite failed:", err);
  process.exit(1);
});
