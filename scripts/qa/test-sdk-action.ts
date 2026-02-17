import { Auth9Client, NotFoundError, ValidationError, AuthenticationError } from '@auth9/core';

const TENANT_ID = '4db48d60-cd25-431d-902a-366a4b9bba10';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  tenantId: TENANT_ID,
});

async function testScenario3_CRU() {
  console.log('=== Scenario 3: SDK CRUD ===');
  
  const action = await client.actions.create({
    name: 'SDK Test Action',
    trigger_id: 'post-login',
    script: 'context.claims = context.claims || {}; context.claims.sdk_test = true; context;',
    enabled: true,
  });
  console.log('Created:', action.id);

  const actions = await client.actions.list();
  console.log('Total actions:', actions.length);

  const retrieved = await client.actions.get(action.id);
  console.log('Retrieved:', retrieved.name);

  const updated = await client.actions.update(action.id, {
    description: 'Updated via SDK',
  });
  console.log('Updated description:', updated.description);

  await client.actions.delete(action.id);
  console.log('Deleted successfully');
  
  return true;
}

async function testScenario4_BatchUpsert() {
  console.log('\n=== Scenario 4: Batch Upsert ===');
  
  const result = await client.actions.batchUpsert([
    {
      name: 'service-a-rule',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_a = true; context;',
    },
    {
      name: 'service-b-rule',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_b = true; context;',
    },
  ]);

  console.log('Created:', result.created.length);
  console.log('Updated:', result.updated.length);
  console.log('Errors:', result.errors.length);
  
  for (const action of [...result.created, ...result.updated]) {
    await client.actions.delete(action.id);
  }
  
  return result.created.length >= 2 && result.errors.length === 0;
}

async function testScenario5_TestAction() {
  console.log('\n=== Scenario 5: Test Action ===');
  
  const action = await client.actions.create({
    name: 'Test Action',
    trigger_id: 'post-login',
    script: `
      if (context.user.email.endsWith("@blocked.com")) {
        throw new Error("Blocked domain");
      }
      context.claims = context.claims || {};
      context.claims.tested = true;
      context;
    `,
  });

  const successResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user-id',
      email: 'user@allowed.com',
      mfa_enabled: false,
    },
    tenant: {
      id: TENANT_ID,
      slug: 'test-tenant',
      name: 'Test Tenant',
    },
    request: {
      ip: '1.2.3.4',
      user_agent: 'Mozilla/5.0',
      timestamp: new Date().toISOString(),
    },
  });
  console.log('Success test:', successResult.success);
  console.log('Modified claims:', successResult.modified_context?.claims);

  const failResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user-id',
      email: 'user@blocked.com',
      mfa_enabled: false,
    },
    tenant: {
      id: TENANT_ID,
      slug: 'test-tenant',
      name: 'Test Tenant',
    },
    request: {
      ip: '1.2.3.4',
      timestamp: new Date().toISOString(),
    },
  });
  console.log('Fail test:', failResult.success);
  console.log('Error:', failResult.error_message);

  await client.actions.delete(action.id);
  
  return successResult.success === true && failResult.success === false;
}

async function testScenario6_Logs() {
  console.log('\n=== Scenario 6: Logs ===');
  
  const action = await client.actions.create({
    name: 'Logging Test',
    trigger_id: 'post-login',
    script: 'context;',
  });

  await client.actions.test(action.id, {
    user: { id: '1', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: TENANT_ID, slug: 'test', name: 'Test' },
    request: { timestamp: new Date().toISOString() },
  });

  const logs = await client.actions.logs({ actionId: action.id, limit: 10 });

  console.log('Total logs:', logs.length);
  console.log('Latest log:', logs[0] ? { 
    id: logs[0].id, 
    action_id: logs[0].action_id, 
    success: logs[0].success,
    duration_ms: logs[0].duration_ms 
  } : 'none');

  await client.actions.delete(action.id);
  
  return logs.length >= 1;
}

async function testScenario7_Stats() {
  console.log('\n=== Scenario 7: Stats ===');
  
  const action = await client.actions.create({
    name: 'Stats Test',
    trigger_id: 'post-login',
    script: 'context;',
  });

  for (let i = 0; i < 5; i++) {
    await client.actions.test(action.id, {
      user: { id: '1', email: 'test@example.com', mfa_enabled: false },
      tenant: { id: TENANT_ID, slug: 'test', name: 'Test' },
      request: { timestamp: new Date().toISOString() },
    });
  }

  const stats = await client.actions.stats(action.id);
  console.log('Execution count:', stats.execution_count);
  console.log('Success rate:', stats.success_rate);
  console.log('Avg duration:', stats.avg_duration_ms);
  console.log('Last 24h:', stats.last_24h_count);

  await client.actions.delete(action.id);
  
  return stats.execution_count >= 5;
}

async function testScenario8_Errors() {
  console.log('\n=== Scenario 8: Error Handling ===');
  
  const errorClient = new Auth9Client({
    baseUrl: 'http://localhost:8080',
    apiKey: 'invalid-token',
    tenantId: TENANT_ID,
  });

  try {
    await errorClient.actions.list();
    console.log('ERROR: Should have thrown AuthenticationError');
  } catch (error) {
    if (error instanceof AuthenticationError) {
      console.log('Authentication failed:', error.message);
    }
  }

  try {
    await client.actions.get('non-existent-id');
    console.log('ERROR: Should have thrown NotFoundError');
  } catch (error) {
    if (error instanceof NotFoundError) {
      console.log('Action not found:', error.message);
    }
  }

  try {
    await client.actions.create({
      name: '',
      trigger_id: 'invalid-trigger',
      script: '',
    } as any);
    console.log('ERROR: Should have thrown ValidationError');
  } catch (error) {
    if (error instanceof ValidationError) {
      console.log('Validation failed:', error.message);
    }
  }
  
  return true;
}

async function testScenario9_Concurrency() {
  console.log('\n=== Scenario 9: Concurrency ===');
  
  const start = Date.now();
  const promises = Array.from({ length: 20 }, (_, i) =>
    client.actions.create({
      name: `Concurrent Action ${i}`,
      trigger_id: 'post-login',
      script: 'context;',
    })
  );

  const results = await Promise.all(promises);
  const duration = Date.now() - start;

  console.log('Created:', results.length);
  console.log('Total time:', duration, 'ms');
  console.log('Avg time per request:', (duration / results.length).toFixed(2), 'ms');

  for (const action of results) {
    await client.actions.delete(action.id);
  }
  
  return results.length === 20 && duration < 5000;
}

async function main() {
  const results: { name: string; pass: boolean; error?: string }[] = [];

  try {
    results.push({ name: 'Scenario 3: CRUD', pass: await testScenario3_CRU() });
  } catch (e: any) {
    results.push({ name: 'Scenario 3: CRUD', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 4: Batch Upsert', pass: await testScenario4_BatchUpsert() });
  } catch (e: any) {
    results.push({ name: 'Scenario 4: Batch Upsert', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 5: Test Action', pass: await testScenario5_TestAction() });
  } catch (e: any) {
    results.push({ name: 'Scenario 5: Test Action', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 6: Logs', pass: await testScenario6_Logs() });
  } catch (e: any) {
    results.push({ name: 'Scenario 6: Logs', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 7: Stats', pass: await testScenario7_Stats() });
  } catch (e: any) {
    results.push({ name: 'Scenario 7: Stats', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 8: Errors', pass: await testScenario8_Errors() });
  } catch (e: any) {
    results.push({ name: 'Scenario 8: Errors', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 9: Concurrency', pass: await testScenario9_Concurrency() });
  } catch (e: any) {
    results.push({ name: 'Scenario 9: Concurrency', pass: false, error: e.message });
  }

  console.log('\n=== SUMMARY ===');
  for (const r of results) {
    console.log(`${r.pass ? '✅' : '❌'} ${r.name}${r.error ? ': ' + r.error : ''}`);
  }
  
  const passed = results.filter(r => r.pass).length;
  const total = results.length;
  console.log(`\nTotal: ${passed}/${total} passed`);
}

main().catch(console.error);
