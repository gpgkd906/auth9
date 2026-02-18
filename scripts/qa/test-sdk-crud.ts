import { Auth9Client } from '@auth9/core';

const TOKEN = process.env.AUTH9_API_KEY!;
const TENANT_ID = process.env.TENANT_ID || '2f7ef923-dbdc-457f-84a2-42efefb77095';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: TOKEN,
  tenantId: TENANT_ID,
});

async function testCRUD() {
  console.log('=== 场景3：TypeScript SDK - 基础CRUD ===');
  
  // 1. 创建 Action
  console.log('1. 创建Action...');
  const action = await client.actions.create({
    name: 'SDK Test Action',
    trigger_id: 'post-login',
    script: 'context.claims = context.claims || {}; context.claims.sdk_test = true; context;',
    enabled: true,
  });
  console.log('Created:', action.id);

  // 2. 获取列表
  console.log('2. 获取列表...');
  const actions = await client.actions.list();
  console.log('Total actions:', actions.length);

  // 3. 获取单个
  console.log('3. 获取单个...');
  const retrieved = await client.actions.get(action.id);
  console.log('Retrieved:', retrieved.name);

  // 4. 更新
  console.log('4. 更新...');
  const updated = await client.actions.update(action.id, {
    description: 'Updated via SDK',
  });
  console.log('Updated description:', updated.description);

  // 5. 删除
  console.log('5. 删除...');
  await client.actions.delete(action.id);
  console.log('Deleted successfully');
}

testCRUD().catch(console.error);