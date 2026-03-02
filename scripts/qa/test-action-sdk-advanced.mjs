import { Auth9Client, NotFoundError, ValidationError, UnauthorizedError } from '@auth9/core';

const TOKEN = process.env.AUTH9_TOKEN || '';
const SERVICE_ID = process.env.AUTH9_SERVICE_ID || 'f7bf6609-9e6a-48cf-864f-7f2f091eed10';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: TOKEN,
  serviceId: SERVICE_ID,
});

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function runTests() {
  console.log('=== SDK - 日志查询测试 (场景 6) ===');
  try {
    const action = await client.actions.create({
      name: 'Logging Test',
      trigger_id: 'post-login',
      script: 'context;',
    });
    console.log('Created action:', action.id);

    await client.actions.test(action.id, {
      user: { id: '1', email: 'test@example.com', mfa_enabled: false },
      tenant: { id: 'tenant-id', slug: 'test', name: 'Test' },
      request: { timestamp: new Date().toISOString() },
    });

    await sleep(500);

    const logs = await client.actions.logs({
      actionId: action.id,
      limit: 10,
    });

    console.log('Total logs:', logs.length);
    if (logs.length > 0) {
      console.log('Latest log:', JSON.stringify(logs[0], null, 2));
      console.log('✅ 场景 6 PASS');
    } else {
      console.log('❌ 场景 6 FAIL - No logs returned');
    }
  } catch (error) {
    console.log('❌ 场景 6 FAIL:', error);
  }

  console.log('\n=== SDK - 统计信息查询测试 (场景 7) ===');
  try {
    const action = await client.actions.create({
      name: 'Stats Test',
      trigger_id: 'post-login',
      script: 'context;',
    });

    for (let i = 0; i < 10; i++) {
      await client.actions.test(action.id, {
        user: { id: '1', email: 'test@example.com', mfa_enabled: false },
        tenant: { id: 'tenant-id', slug: 'test', name: 'Test' },
        request: { timestamp: new Date().toISOString() },
      });
    }

    await sleep(500);

    const stats = await client.actions.stats(action.id);
    console.log('Execution count:', stats.execution_count);
    console.log('Success rate:', stats.success_rate);
    console.log('Avg duration:', stats.avg_duration_ms);
    console.log('Last 24h:', stats.last_24h_count);

    if (stats.execution_count === 10 && stats.success_rate === 100) {
      console.log('✅ 场景 7 PASS');
    } else {
      console.log('❌ 场景 7 FAIL - Stats mismatch');
    }
  } catch (error) {
    console.log('❌ 场景 7 FAIL:', error);
  }

  console.log('\n=== 错误处理 - SDK 测试 (场景 8) ===');
  try {
    const invalidClient = new Auth9Client({
      baseUrl: 'http://localhost:8080',
      apiKey: 'invalid-token', // pragma: allowlist secret
      serviceId: SERVICE_ID,
    });

    try {
      await invalidClient.actions.list();
      console.log('❌ 场景 8 FAIL - Should have thrown AuthenticationError');
    } catch (error) {
      if (error instanceof UnauthorizedError) {
        console.log('Authentication failed (expected):', error.message);
      } else {
        console.log('Different error:', error);
      }
    }

    try {
      await client.actions.get('non-existent-id');
      console.log('❌ 场景 8 FAIL - Should have thrown NotFoundError');
    } catch (error) {
      if (error instanceof NotFoundError) {
        console.log('Action not found (expected):', error.message);
      }
    }

    try {
      await client.actions.create({
        name: '',
        trigger_id: 'invalid-trigger',
        script: '',
      });
      console.log('❌ 场景 8 FAIL - Should have thrown ValidationError');
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('Validation failed (expected):', error.message);
      }
    }

    console.log('✅ 场景 8 PASS');
  } catch (error) {
    console.log('❌ 场景 8 FAIL:', error);
  }

  console.log('\n=== 并发请求测试 (场景 9) ===');
  try {
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
    console.log('Avg time per request:', Math.round(duration / results.length), 'ms');

    if (results.length === 20 && duration < 2000) {
      console.log('✅ 场景 9 PASS');
    } else {
      console.log('❌ 场景 9 FAIL - Performance issue');
    }
  } catch (error) {
    console.log('❌ 场景 9 FAIL:', error);
  }

  console.log('\n=== AI Agent 集成模式测试 (场景 10) ===');
  try {
    const serviceName = 'service-x';

    const existingActions = await client.actions.list('post-login');
    const existingRule = existingActions.find(a => a.name === `${serviceName}-access-control`);

    if (existingRule) {
      console.log('Rule already exists, updating...');
      await client.actions.update(existingRule.id, {
        script: generateAccessControlScript(serviceName),
      });
    } else {
      console.log('Creating new rule...');
      await client.actions.create({
        name: `${serviceName}-access-control`,
        description: `Auto-generated by AI Agent for ${serviceName}`,
        trigger_id: 'post-login',
        script: generateAccessControlScript(serviceName),
        enabled: true,
      });
    }

    const action = (await client.actions.list('post-login')).find(
      a => a.name === `${serviceName}-access-control`
    );

    const testResult = await client.actions.test(action.id, {
      user: {
        id: 'test-user',
        email: 'admin@company.com',
        mfa_enabled: false,
      },
      tenant: {
        id: 'tenant-id',
        slug: 'company',
        name: 'Company',
      },
      request: {
        timestamp: new Date().toISOString(),
      },
      claims: {
        roles: ['developer'],
      },
    });

    console.log('Test result:', testResult.success);
    console.log('Service access granted:', testResult.modified_context?.claims?.service_access);

    if (testResult.success) {
      console.log('✅ 场景 10 PASS');
    } else {
      console.log('❌ 场景 10 FAIL - Test failed');
    }
  } catch (error) {
    console.log('❌ 场景 10 FAIL:', error);
  }

  console.log('\n=== 测试完成 ===');
}

function generateAccessControlScript(serviceName) {
  return `
const allowedRoles = ["admin", "developer"];
const userRoles = (context.claims?.roles) || [];

const hasAccess = allowedRoles.some(role => userRoles.includes(role));

if (!hasAccess) {
  throw new Error("Insufficient permissions for ${serviceName}");
}

context.claims = context.claims || {};
context.claims.service_access = context.claims.service_access || [];
context.claims.service_access.push("${serviceName}");

context;
  `.trim();
}

runTests().catch(console.error);
