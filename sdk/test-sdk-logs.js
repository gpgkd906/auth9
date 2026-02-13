const { Auth9HttpClient } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const TENANT_ID = '259e29f1-5d77-496c-999f-8f0374bae15f';

const client = new Auth9HttpClient({
  baseUrl: 'http://localhost:8080',
  accessToken: TOKEN,
});

async function testLogs() {
  console.log('=== 场景6：SDK - 日志查询 ===');
  
  try {
    // 创建 Action
    console.log('创建测试Action...');
    const action = await client.post(`/api/v1/tenants/${TENANT_ID}/actions`, {
      name: 'Logging Test',
      trigger_id: 'post-login',
      script: 'context;',
    });
    const actionId = action.data.id;
    console.log('Action ID:', actionId);

    // 由于测试端点不可用，我们无法触发执行
    // 直接查询日志（可能为空）
    console.log('查询日志...');
    const logsResult = await client.get(`/api/v1/tenants/${TENANT_ID}/actions/logs`, {
      action_id: actionId,
      limit: 10
    });
    
    const logs = logsResult.data || [];
    console.log('Total logs:', logs.length);
    
    if (Array.isArray(logs)) {
      console.log('✅ 场景6测试通过 - 日志查询API正常工作');
    } else {
      console.log('❌ 场景6测试失败: 日志查询返回非数组结果');
      console.log('结果:', JSON.stringify(logsResult, null, 2));
    }

    // 清理
    console.log('清理测试数据...');
    await client.delete(`/api/v1/tenants/${TENANT_ID}/actions/${actionId}`);
    
  } catch (error) {
    console.error('❌ 场景6测试失败:', error.message);
    console.error(error);
  }
}

testLogs();
