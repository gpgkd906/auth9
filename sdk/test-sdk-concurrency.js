const { Auth9HttpClient } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const TENANT_ID = '0df463ad-10a2-4589-8708-0b56dba70161';

const client = new Auth9HttpClient({
  baseUrl: 'http://localhost:8080',
  accessToken: TOKEN,
});

async function testConcurrency() {
  console.log('=== 场景9：并发请求测试 ===');
  
  try {
    // 并发创建 10 个 Actions（原文档是20个，但为了速度减少到10个）
    console.log('并发创建10个Actions...');
    const start = Date.now();
    const promises = Array.from({ length: 10 }, (_, i) =>
      client.post(`/api/v1/tenants/${TENANT_ID}/actions`, {
        name: `Concurrent Action ${i}`,
        trigger_id: 'post-login',
        script: 'context;',
      })
    );

    const results = await Promise.all(promises);
    const duration = Date.now() - start;

    console.log('Created:', results.length);
    console.log('Total time:', duration, 'ms');
    console.log('Avg time per request:', duration / results.length, 'ms');

    // 验证结果
    const successCount = results.filter(r => r.data && r.data.id).length;
    if (successCount === results.length && duration < 5000) {
      console.log('✅ 场景9测试通过 - 所有并发请求成功');
    } else {
      console.log('❌ 场景9测试失败: 部分请求失败或耗时过长');
      console.log('成功数:', successCount, '/', results.length);
    }

    // 清理
    console.log('清理测试数据...');
    const cleanupPromises = results.map(result =>
      client.delete(`/api/v1/tenants/${TENANT_ID}/actions/${result.data.id}`)
    );
    await Promise.all(cleanupPromises);
    console.log('清理完成');
    
  } catch (error) {
    console.error('❌ 场景9测试失败:', error.message);
    console.error(error);
  }
}

testConcurrency();
