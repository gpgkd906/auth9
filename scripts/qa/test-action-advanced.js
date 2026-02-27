const { execSync } = require('child_process');
const TOKEN = execSync('.claude/skills/tools/gen-admin-token.sh', { encoding: 'utf8' }).trim();
const BASE_URL = 'http://localhost:8080';
const DEMO_TENANT_ID = 'a793666b-248a-44a7-8096-cb968706c71a';
const DEMO_SERVICE_ID = '7d4bfb87-b24f-43fe-9e6e-100f0aaae5b2';

const headers = {
  'Authorization': `Bearer ${TOKEN}`,
  'Content-Type': 'application/json',
};

async function request(method, path, body = null) {
  const options = { method, headers };
  if (body) options.body = JSON.stringify(body);
  const res = await fetch(`${BASE_URL}${path}`, options);
  const data = await res.json();
  if (!res.ok) throw new Error(`${res.status}: ${JSON.stringify(data)}`);
  return data;
}

async function cleanup(actionId) {
  try {
    await request('DELETE', `/api/v1/services/${DEMO_SERVICE_ID}/actions/${actionId}`);
  } catch (e) {}
}

async function testScenario5_Timeout() {
  console.log('\n=== Scenario 5: Action Timeout Control ===');
  
  const script = `
    const start = Date.now();
    while (Date.now() - start < 2000) {}
    context;
  `;
  
  const action = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions`, {
    name: 'Timeout Test Action',
    trigger_id: 'post-login',
    script,
    enabled: true,
    timeout_ms: 1000,
    strict_mode: true,
  });
  console.log('Created action:', action.id);

  const result = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions/${action.id}/test`, {
    user: { id: 'test-user', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: DEMO_TENANT_ID, slug: 'demo', name: 'Demo' },
    request: { ip: '1.2.3.4', user_agent: 'test', timestamp: new Date().toISOString() },
  });
  
  console.log('Test result:', { success: result.success, error: result.error_message, duration: result.duration_ms });
  
  await cleanup(action.id);
  
  if (!result.success && (result.error_message?.includes('timeout') || result.error_message?.includes('exceeded'))) {
    console.log('✅ PASS: Action timed out as expected');
    return true;
  } else {
    console.log('❌ FAIL: Expected timeout error');
    return false;
  }
}

async function testScenario6_DisabledAction() {
  console.log('\n=== Scenario 6: Disabled Action Not Executed ===');
  
  const action = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions`, {
    name: 'Disabled Test Action',
    trigger_id: 'post-login',
    script: 'context.claims = context.claims || {}; context.claims.disabled_test = true; context;',
    enabled: false,
  });
  console.log('Created disabled action:', action.id);

  const result = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions/${action.id}/test`, {
    user: { id: 'test-user', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: DEMO_TENANT_ID, slug: 'demo', name: 'Demo' },
    request: { ip: '1.2.3.4', user_agent: 'test', timestamp: new Date().toISOString() },
  });
  
  console.log('Test result:', result);
  
  await cleanup(action.id);
  
  if (result.success === false && result.skipped === true) {
    console.log('✅ PASS: Disabled action was skipped');
    return true;
  } else {
    console.log('❌ FAIL: Expected action to be skipped');
    return false;
  }
}

async function testScenario7_ContextValidation() {
  console.log('\n=== Scenario 7: Action Context Validation ===');
  
  const script = `
    if (!context.user || !context.tenant || !context.request) {
      throw new Error("Context incomplete");
    }
    if (!context.user.email || !context.user.id) {
      throw new Error("User info missing");
    }
    context.claims = context.claims || {};
    context.claims.context_validated = true;
    context;
  `;
  
  const action = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions`, {
    name: 'Context Validation Action',
    trigger_id: 'post-login',
    script,
    enabled: true,
  });
  console.log('Created action:', action.id);

  const result = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions/${action.id}/test`, {
    user: { id: 'test-user-id', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: DEMO_TENANT_ID, slug: 'demo', name: 'Demo' },
    request: { ip: '1.2.3.4', user_agent: 'test', timestamp: new Date().toISOString() },
  });
  
  console.log('Test result:', { success: result.success, claims: result.modified_context?.claims });
  
  await cleanup(action.id);
  
  if (result.success && result.modified_context?.claims?.context_validated === true) {
    console.log('✅ PASS: Context validated successfully');
    return true;
  } else {
    console.log('❌ FAIL: Context validation failed');
    return false;
  }
}

async function testScenario8_ServiceIsolation() {
  console.log('\n=== Scenario 8: Service Isolation ===');
  
  // Create second service
  const service2 = await request('POST', `/api/v1/tenants/${DEMO_TENANT_ID}/services`, {
    name: 'Test Service B',
  });
  console.log('Created service B:', service2.id);

  const scriptA = 'context.claims = context.claims || {}; context.claims.service_a_action = true; context;';
  const scriptB = 'context.claims = context.claims || {}; context.claims.service_b_action = true; context;';
  
  const actionA = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions`, {
    name: 'Service A Action',
    trigger_id: 'post-login',
    script: scriptA,
    enabled: true,
  });
  
  const actionB = await request('POST', `/api/v1/services/${service2.id}/actions`, {
    name: 'Service B Action',
    trigger_id: 'post-login',
    script: scriptB,
    enabled: true,
  });
  
  console.log('Created action A:', actionA.id, 'service:', actionA.service_id);
  console.log('Created action B:', actionB.id, 'service:', actionB.service_id);

  const resultA = await request('POST', `/api/v1/services/${DEMO_SERVICE_ID}/actions/${actionA.id}/test`, {
    user: { id: 'test-user', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: DEMO_TENANT_ID, slug: 'demo', name: 'Demo' },
    request: { ip: '1.2.3.4', user_agent: 'test', timestamp: new Date().toISOString() },
  });

  const resultB = await request('POST', `/api/v1/services/${service2.id}/actions/${actionB.id}/test`, {
    user: { id: 'test-user', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: DEMO_TENANT_ID, slug: 'demo', name: 'Demo' },
    request: { ip: '1.2.3.4', user_agent: 'test', timestamp: new Date().toISOString() },
  });
  
  console.log('Result A:', { success: resultA.success, serviceId: resultA.service_id, claims: resultA.modified_context?.claims });
  console.log('Result B:', { success: resultB.success, serviceId: resultB.service_id, claims: resultB.modified_context?.claims });

  await cleanup(actionA.id);
  await cleanup(actionB.id);
  await request('DELETE', `/api/v1/services/${service2.id}`);

  const passA = resultA.success && resultA.modified_context?.claims?.service_a_action === true && resultA.service_id === DEMO_SERVICE_ID;
  const passB = resultB.success && resultB.modified_context?.claims?.service_b_action === true && resultB.service_id === service2.id;
  
  if (passA && passB) {
    console.log('✅ PASS: Service isolation works correctly');
    return true;
  } else {
    console.log('❌ FAIL: Service isolation issue');
    return false;
  }
}

async function main() {
  const results = [];
  
  try {
    results.push({ name: 'Scenario 5: Timeout Control', pass: await testScenario5_Timeout() });
  } catch (e) {
    console.log('❌ Scenario 5 error:', e.message);
    results.push({ name: 'Scenario 5: Timeout Control', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 6: Disabled Action', pass: await testScenario6_DisabledAction() });
  } catch (e) {
    console.log('❌ Scenario 6 error:', e.message);
    results.push({ name: 'Scenario 6: Disabled Action', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 7: Context Validation', pass: await testScenario7_ContextValidation() });
  } catch (e) {
    console.log('❌ Scenario 7 error:', e.message);
    results.push({ name: 'Scenario 7: Context Validation', pass: false, error: e.message });
  }

  try {
    results.push({ name: 'Scenario 8: Service Isolation', pass: await testScenario8_ServiceIsolation() });
  } catch (e) {
    console.log('❌ Scenario 8 error:', e.message);
    results.push({ name: 'Scenario 8: Service Isolation', pass: false, error: e.message });
  }

  console.log('\n=== SUMMARY ===');
  let passed = 0;
  for (const r of results) {
    console.log(`${r.pass ? '✅' : '❌'} ${r.name}${r.error ? ': ' + r.error : ''}`);
    if (r.pass) passed++;
  }
  console.log(`\nTotal: ${passed}/${results.length} passed`);
}

main().catch(console.error);
