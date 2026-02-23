const { Auth9HttpClient } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const SERVICE_ID = '040502d5-e073-4ba2-ae21-4ca8069f0415';

const client = new Auth9HttpClient({
  baseUrl: 'http://localhost:8080',
  accessToken: TOKEN,
});

async function testCRUD() {
  console.log('=== 场景3：TypeScript SDK - 基础CRUD ===');
  
  try {
    // 1. 创建 Action
    console.log('1. 创建Action...');
    const createResult = await client.post(`/api/v1/services/${SERVICE_ID}/actions`, {
      name: 'SDK Test Action',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.sdk_test = true; context;',
      enabled: true,
    });
    const action = createResult.data;
    console.log('Created:', action.id);

    // 2. 获取列表
    console.log('2. 获取列表...');
    const listResult = await client.get(`/api/v1/services/${SERVICE_ID}/actions`);
    const actions = listResult.data;
    console.log('Total actions:', actions.length);

    // 3. 获取单个
    console.log('3. 获取单个...');
    const getResult = await client.get(`/api/v1/services/${SERVICE_ID}/actions/${action.id}`);
    const retrieved = getResult.data;
    console.log('Retrieved:', retrieved.name);

    // 4. 更新
    console.log('4. 更新...');
    const updateResult = await client.patch(`/api/v1/services/${SERVICE_ID}/actions/${action.id}`, {
      description: 'Updated via SDK',
    });
    const updated = updateResult.data;
    console.log('Updated description:', updated.description);

    // 5. 删除
    console.log('5. 删除...');
    await client.delete(`/api/v1/services/${SERVICE_ID}/actions/${action.id}`);
    console.log('Deleted successfully');
    
    console.log('✅ 场景3测试通过');
  } catch (error) {
    console.error('❌ 场景3测试失败:', error.message);
    console.error(error);
  }
}

testCRUD();
