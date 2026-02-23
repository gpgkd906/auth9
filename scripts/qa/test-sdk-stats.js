const { Auth9HttpClient } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const SERVICE_ID = '040502d5-e073-4ba2-ae21-4ca8069f0415';

const client = new Auth9HttpClient({
  baseUrl: 'http://localhost:8080',
  accessToken: TOKEN,
});

async function testStats() {
  console.log('=== 场景7：SDK - 统计信息查询 ===');
  
  try {
    // 创建 Action
    console.log('创建测试Action...');
    const action = await client.post(`/api/v1/services/${SERVICE_ID}/actions`, {
      name: 'Stats Test',
      trigger_id: 'post-login',
      script: 'context;',
    });
    const actionId = action.data.id;
    console.log('Action ID:', actionId);

    // 查询统计
    console.log('查询统计信息...');
    const statsResult = await client.get(`/api/v1/services/${SERVICE_ID}/actions/${actionId}/stats`);
    
    const stats = statsResult.data;
    console.log('Execution count:', stats?.executionCount || 0);
    console.log('Error count:', stats?.errorCount || 0);
    console.log('Avg duration:', stats?.avgDurationMs || 0);
    console.log('Last 24h:', stats?.last24hCount || 0);

    // 验证结果
    if (stats && typeof stats.executionCount === 'number') {
      console.log('✅ 场景7测试通过 - 统计查询API正常工作');
    } else {
      console.log('❌ 场景7测试失败: 统计查询返回无效结果');
      console.log('结果:', JSON.stringify(statsResult, null, 2));
    }

    // 清理
    console.log('清理测试数据...');
    await client.delete(`/api/v1/services/${SERVICE_ID}/actions/${actionId}`);
    
  } catch (error) {
    console.error('❌ 场景7测试失败:', error.message);
    console.error(error);
  }
}

testStats();
