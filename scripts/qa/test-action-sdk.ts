import { Auth9Client } from '/Volumes/Yotta/auth9/auth9-portal/node_modules/@auth9/core/dist/index.js';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  serviceId: '70356552-776b-4d66-8b18-1d7328239738',
});

async function testCRUD() {
  console.log('=== 场景 3: TypeScript SDK - 基础 CRUD ===');
  
  // 1. 创建 Action
  const action = await client.actions.create({
    name: 'SDK Test Action',
    trigger_id: 'post-login',
    script: 'context.claims = context.claims || {}; context.claims.sdk_test = true; context;',
    enabled: true,
  });
  console.log('Created:', action.id);

  // 2. 获取列表
  const actions = await client.actions.list();
  console.log('Total actions:', actions.length);

  // 3. 获取单个
  const retrieved = await client.actions.get(action.id);
  console.log('Retrieved:', retrieved.name);

  // 4. 更新
  const updated = await client.actions.update(action.id, {
    description: 'Updated via SDK',
  });
  console.log('Updated description:', updated.description);

  // 5. 删除
  await client.actions.delete(action.id);
  console.log('Deleted successfully');
  
  console.log('\n=== 场景 3: PASS ===');
}

async function testBatchUpsert() {
  console.log('\n=== 场景 4: SDK - 批量操作（Batch Upsert） ===');
  
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
    {
      name: 'service-c-rule',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_c = true; context;',
    },
  ]);

  console.log('Created:', result.created.length);
  console.log('Updated:', result.updated.length);
  console.log('Errors:', result.errors.length);
  
  // 清理
  for (const action of [...result.created, ...result.updated]) {
    await client.actions.delete(action.id);
  }
  
  console.log('\n=== 场景 4: PASS ===');
}

async function testAction() {
  console.log('\n=== 场景 5: SDK - 测试 Action（Test Endpoint） ===');
  
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

  // 测试：允许的邮箱域
  const successResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user-id',
      email: 'user@allowed.com',
      mfa_enabled: false,
    },
    tenant: {
      id: '3427371a-b594-4d47-9c67-d876cab0522b',
      slug: 'demo',
      name: 'Demo Organization',
    },
    request: {
      ip: '1.2.3.4',
      user_agent: 'Mozilla/5.0',
      timestamp: new Date().toISOString(),
    },
  });
  console.log('Success test:', successResult.success);
  console.log('Modified claims:', JSON.stringify(successResult.modified_context?.claims));

  // 测试：阻止的邮箱域
  const failResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user-id',
      email: 'user@blocked.com',
      mfa_enabled: false,
    },
    tenant: {
      id: '3427371a-b594-4d47-9c67-d876cab0522b',
      slug: 'demo',
      name: 'Demo Organization',
    },
    request: {
      ip: '1.2.3.4',
      timestamp: new Date().toISOString(),
    },
  });
  console.log('Fail test:', failResult.success);
  console.log('Error:', failResult.error_message);
  
  // 清理
  await client.actions.delete(action.id);
  
  console.log('\n=== 场景 5: PASS ===');
}

async function main() {
  try {
    await testCRUD();
    await testBatchUpsert();
    await testAction();
    console.log('\n✅ 所有测试通过！');
  } catch (error) {
    console.error('Test failed:', error);
    process.exit(1);
  }
}

main();
