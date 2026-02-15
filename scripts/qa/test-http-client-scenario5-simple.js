import { Auth9HttpClient } from './packages/core/dist/index.js';

const TOKEN = process.env.AUTH9_TOKEN || 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3YTYxZGJiOC1mOTM3LTQ5YjktYTlkMy1iMWMzNWZiNTI4ZmUiLCJlbWFpbCI6ImFkbWluQGF1dGg5LmxvY2FsIiwibmFtZSI6IkFkbWluIFVzZXIiLCJpc3MiOiJodHRwOi8vbG9jYWxob3N0OjgwODAiLCJhdWQiOiJhdXRoOSIsImlhdCI6MTc3MTEwNDYwMSwiZXhwIjoxNzcxMTA4MjAxfQ.kLQQp-4Rp6DwTIv528mH4RqOCSbfrsIwHD8YdXpSLaYpSul3pBmL7hFIUnIEEYHYOF2WhGJNPQ9pnEg_1u2AG72uOarRWQ_rlALnEmN2wqA1YNNaeKEb2zXmF1r6Bu21sHgm2BhK99P8s9IAhEYha2HHrDw194j2m8DrjQBve5Df1GvLFhnUOiZAbIpdA6LwJbBx97AoFIy3jWDnK9Ru-pPE84BTJIFmht3LHOkA4ZC0bP2BR733kkswtYCq9XUKHbh3U6stxOOjfkcIIut6yXNemk1dWB73UFyGiShot9n96x_IGYt3Z2E7IMtcoTmSsNidjf8DW31OFYf0H3lcYA';

async function testScenario5() {
  console.log('=== 场景5：DELETE请求与204 No Content处理 ===');
  
  // 使用原始fetch测试，因为HttpClient需要特殊header
  const baseUrl = 'http://localhost:8080';
  
  // 1. 先创建一个测试租户
  console.log('1. 创建测试租户...');
  const createResponse = await fetch(`${baseUrl}/api/v1/tenants`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${TOKEN}`,
    },
    body: JSON.stringify({
      name: 'To Delete Test Simple',
      slug: 'to-delete-sdk-test-simple',
    }),
  });
  
  const created = await createResponse.json();
  const createdTenantId = created.data.id;
  console.log('创建成功，ID:', createdTenantId);
  
  // 2. 删除该租户（204 No Content）
  console.log('\n2. 删除该租户...');
  const deleteResponse = await fetch(`${baseUrl}/api/v1/tenants/${createdTenantId}`, {
    method: 'DELETE',
    headers: {
      'Authorization': `Bearer ${TOKEN}`,
      'X-Confirm-Destructive': 'true',
    },
  });
  
  console.log('删除响应状态:', deleteResponse.status);
  console.log('删除响应状态文本:', deleteResponse.statusText);
  
  // 验证204 No Content
  if (deleteResponse.status === 204) {
    console.log('✅ 正确：返回204 No Content');
    
    // 验证响应体为空
    const responseText = await deleteResponse.text();
    console.log('响应体内容:', responseText === '' ? '(空字符串)' : responseText);
    console.log('响应体长度:', responseText.length);
  } else {
    console.log('❌ 错误：期望204 No Content，实际得到:', deleteResponse.status);
  }
  
  // 3. 验证再次GET该租户返回404
  console.log('\n3. 验证再次GET该租户返回404...');
  const getResponse = await fetch(`${baseUrl}/api/v1/tenants/${createdTenantId}`, {
    headers: {
      'Authorization': `Bearer ${TOKEN}`,
    },
  });
  
  if (getResponse.status === 404) {
    console.log('✅ 正确：租户已删除，返回404');
  } else {
    console.log('❌ 错误：期望404，实际得到:', getResponse.status);
  }
  
  // 4. 验证数据库状态
  console.log('\n4. 验证数据库状态...');
  console.log('请运行: SELECT id FROM tenants WHERE slug = \'to-delete-sdk-test-simple\';');
  console.log('预期: 无记录');
}

testScenario5().catch(console.error);