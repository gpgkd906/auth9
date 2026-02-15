import { Auth9HttpClient } from './packages/core/dist/index.js';
import { NotFoundError, UnauthorizedError, ConflictError } from './packages/core/dist/index.js';

const TOKEN = process.env.AUTH9_TOKEN || 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3YTYxZGJiOC1mOTM3LTQ5YjktYTlkMy1iMWMzNWZiNTI4ZmUiLCJlbWFpbCI6ImFkbWluQGF1dGg5LmxvY2FsIiwibmFtZSI6IkFkbWluIFVzZXIiLCJpc3MiOiJodHRwOi8vbG9jYWxob3N0OjgwODAiLCJhdWQiOiJhdXRoOSIsImlhdCI6MTc3MTEwNDYwMSwiZXhwIjoxNzcxMTA4MjAxfQ.kLQQp-4Rp6DwTIv528mH4RqOCSbfrsIwHD8YdXpSLaYpSul3pBmL7hFIUnIEEYHYOF2WhGJNPQ9pnEg_1u2AG72uOarRWQ_rlALnEmN2wqA1YNNaeKEb2zXmF1r6Bu21sHgm2BhK99P8s9IAhEYha2HHrDw194j2m8DrjQBve5Df1GvLFhnUOiZAbIpdA6LwJbBx97AoFIy3jWDnK9Ru-pPE84BTJIFmht3LHOkA4ZC0bP2BR733kkswtYCq9XUKHbh3U6stxOOjfkcIIut6yXNemk1dWB73UFyGiShot9n96x_IGYt3Z2E7IMtcoTmSsNidjf8DW31OFYf0H3lcYA';

async function testScenario3() {
  console.log('=== 场景3：HTTP错误映射到类型化异常 ===');
  
  const client = new Auth9HttpClient({
    baseUrl: 'http://localhost:8080',
    accessToken: TOKEN,
  });

  // 1. 触发404错误
  console.log('\n1. 触发404错误（不存在的资源）:');
  try {
    await client.get('/api/v1/tenants/non-existent-id-123456');
  } catch (err) {
    console.log('err instanceof NotFoundError:', err instanceof NotFoundError);
    console.log('err.statusCode:', err.statusCode);
    console.log('err.code:', err.code);
    console.log('err.message:', err.message);
  }

  // 2. 触发401错误（无token）
  console.log('\n2. 触发401错误（无token）:');
  const noAuthClient = new Auth9HttpClient({ baseUrl: 'http://localhost:8080' });
  try {
    await noAuthClient.get('/api/v1/tenants');
  } catch (err) {
    console.log('err instanceof UnauthorizedError:', err instanceof UnauthorizedError);
    console.log('err.statusCode:', err.statusCode);
    console.log('err.message:', err.message);
  }

  // 3. 触发409冲突（重复slug）
  console.log('\n3. 触发409冲突（重复slug）:');
  try {
    // 先创建一个租户
    const created = await client.post('/api/v1/tenants', {
      name: 'Conflict Test',
      slug: 'conflict-test-slug',
    });
    console.log('创建第一个租户成功，ID:', created.data.id);
    
    // 尝试用相同的slug创建第二个租户
    await client.post('/api/v1/tenants', {
      name: 'Conflict Test Duplicate',
      slug: 'conflict-test-slug',
    });
  } catch (err) {
    console.log('err instanceof ConflictError:', err instanceof ConflictError);
    console.log('err.statusCode:', err.statusCode);
    console.log('err.code:', err.code);
    console.log('err.message:', err.message);
    
    // 清理测试数据
    if (err instanceof ConflictError) {
      console.log('\n清理测试数据...');
      try {
        // 获取刚创建的租户ID并删除
        const tenants = await client.get('/api/v1/tenants?slug=conflict-test-slug');
        if (tenants.data && tenants.data.length > 0) {
          const tenantId = tenants.data[0].id;
          await client.delete(`/api/v1/tenants/${tenantId}`);
          console.log('清理成功');
        }
      } catch (cleanupErr) {
        console.log('清理失败:', cleanupErr.message);
      }
    }
  }
}

testScenario3().catch(console.error);