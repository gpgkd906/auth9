import { Auth9Client } from "./src/index.js";

const TOKEN = process.env.AUTH9_API_KEY!;

if (!TOKEN) {
  console.error("Please set AUTH9_API_KEY environment variable");
  process.exit(1);
}

const client = new Auth9Client({
  baseUrl: "http://localhost:8080",
  apiKey: TOKEN,
  tenantId: "0b210fd8-8517-4d55-9414-63a1cce01551",
  serviceId: "5f6836f2-a4bd-43da-9d38-71b1c57cbc37",
});

async function testMemoryExhaustion() {
  console.log("=== Scenario 5: Memory Exhaustion Test ===");
  
  const action = await client.actions.create({
    name: "Memory Exhaustion Test",
    trigger_id: "post-login",
    script: `const arr = [];
for (let i = 0; i < 100000000; i++) {
  arr.push(new Array(1000).fill("x"));
}
context.claims = context.claims || {};
context.claims.allocated = arr.length;
context;`,
    enabled: true,
  });

  console.log("Created action:", action.id);

  try {
    const result = await client.actions.test(action.id, {
      user: { id: "test-user-id", email: "test@example.com", mfa_enabled: false },
      tenant: { id: "0b210fd8-8517-4d55-9414-63a1cce01551", slug: "demo", name: "Demo Organization" },
      request: { ip: "1.2.3.4", user_agent: "Mozilla/5.0", timestamp: new Date().toISOString() },
    });

    console.log("Test result:", JSON.stringify(result, null, 2));
    
    if (result.success) {
      console.log("FAIL: Action should have failed with memory error");
    } else {
      console.log("PASS: Action failed as expected");
      console.log("Error message:", result.error_message);
    }
  } catch (e: any) {
    console.log("Exception:", e.message);
  }

  await client.actions.delete(action.id);
}

testMemoryExhaustion().catch(console.error);
