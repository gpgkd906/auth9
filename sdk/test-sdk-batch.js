const { Auth9HttpClient } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const TENANT_ID = '259e29f1-5d77-496c-999f-8f0374bae15f';

const client = new Auth9HttpClient({
  baseUrl: 'http://localhost:8080',
  accessToken: TOKEN,
});

async function testBatchUpsert() {
  console.log('=== 场景4：SDK - 批量操作（Batch Upsert） ===');
  
  try {
    // 先创建一个现有的Action用于更新
    console.log('创建现有Action用于更新...');
    const existingAction = await client.post(`/api/v1/tenants/${TENANT_ID}/actions`, {
      name: 'service-c-rule',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_c = "old"; context;',
    });
    const existingActionId = existingAction.data.id;
    console.log('现有Action ID:', existingActionId);

    // 批量创建/更新 Actions
    console.log('执行批量操作...');
    const result = await client.post(`/api/v1/tenants/${TENANT_ID}/actions/batch`, {
      actions: [
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
          id: existingActionId, // 更新现有 Action
          name: 'updated-service-c',
          trigger_id: 'post-login',
          script: 'context.claims = context.claims || {}; context.claims.service_c = "updated"; context;',
        },
      ]
    });

    console.log('Created:', result.created?.length || 0);
    console.log('Updated:', result.updated?.length || 0);
    console.log('Errors:', result.errors?.length || 0);

    // 验证结果
    if ((result.created?.length || 0) >= 2 && (result.updated?.length || 0) >= 1 && (result.errors?.length || 0) === 0) {
      console.log('✅ 场景4测试通过');
    } else {
      console.log('❌ 场景4测试失败: 结果不符合预期');
      console.log('完整结果:', JSON.stringify(result, null, 2));
    }

    // 清理
    console.log('清理测试数据...');
    const listResult = await client.get(`/api/v1/tenants/${TENANT_ID}/actions`);
    const actions = listResult.data;
    for (const action of actions) {
      if (action.name.includes('service-') || action.name.includes('updated-service-')) {
        await client.delete(`/api/v1/tenants/${TENANT_ID}/actions/${action.id}`);
      }
    }
    
  } catch (error) {
    console.error('❌ 场景4测试失败:', error.message);
    console.error(error);
  }
}

testBatchUpsert();
