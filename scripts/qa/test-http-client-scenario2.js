import { Auth9HttpClient } from './packages/core/dist/index.js';

const TOKEN = process.env.AUTH9_TOKEN || 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3YTYxZGJiOC1mOTM3LTQ5YjktYTlkMy1iMWMzNWZiNTI4ZmUiLCJlbWFpbCI6ImFkbWluQGF1dGg5LmxvY2FsIiwibmFtZSI6IkFkbWluIFVzZXIiLCJpc3MiOiJodHRwOi8vbG9jYWxob3N0OjgwODAiLCJhdWQiOiJhdXRoOSIsImlhdCI6MTc3MTEwNDYwMSwiZXhwIjoxNzcxMTA4MjAxfQ.kLQQp-4Rp6DwTIv528mH4RqOCSbfrsIwHD8YdXpSLaYpSul3pBmL7hFIUnIEEYHYOF2WhGJNPQ9pnEg_1u2AG72uOarRWQ_rlALnEmN2wqA1YNNaeKEb2zXmF1r6Bu21sHgm2BhK99P8s9IAhEYha2HHrDw194j2m8DrjQBve5Df1GvLFhnUOiZAbIpdA6LwJbBx97AoFIy3jWDnK9Ru-pPE84BTJIFmht3LHOkA4ZC0bP2BR733kkswtYCq9XUKHbh3U6stxOOjfkcIIut6yXNemk1dWB73UFyGiShot9n96x_IGYt3Z2E7IMtcoTmSsNidjf8DW31OFYf0H3lcYA';

async function testScenario2() {
  console.log('=== 场景2：POST请求体自动snake_case转换 ===');
  
  const client = new Auth9HttpClient({
    baseUrl: 'http://localhost:8080',
    accessToken: TOKEN,
  });

  // 1. 使用camelCase参数创建租户
  console.log('1. 使用camelCase参数创建租户...');
  const result = await client.post('/api/v1/tenants', {
    name: 'SDK Test Tenant Scenario2',
    slug: 'sdk-test-scenario2',
    logoUrl: 'https://example.com/logo-scenario2.png',
  });

  console.log('创建结果:', result);
  
  // 2. 验证返回值的key为camelCase
  console.log('\n2. 验证返回值的key为camelCase:');
  console.log('result.data.logoUrl:', result.data.logoUrl);
  console.log('result.data.createdAt:', result.data.createdAt);
  
  // 3. 验证数据库中的实际数据
  console.log('\n3. 验证数据库中的实际数据（snake_case）:');
  console.log('请运行: SELECT id, name, slug, logo_url FROM tenants WHERE slug = \'sdk-test-scenario2\';');
  
  // 4. 清理测试数据
  console.log('\n4. 清理测试数据...');
  try {
    await client.delete(`/api/v1/tenants/${result.data.id}`);
    console.log('清理成功');
  } catch (error) {
    console.log('清理失败（可能租户不存在）:', error.message);
  }
}

testScenario2().catch(console.error);