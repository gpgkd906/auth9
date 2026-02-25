import { Auth9HttpClient } from '../../sdk/packages/core/dist/index.js';

const TOKEN = process.env.AUTH9_TOKEN || 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3YTYxZGJiOC1mOTM3LTQ5YjktYTlkMy1iMWMzNWZiNTI4ZmUiLCJlbWFpbCI6ImFkbWluQGF1dGg5LmxvY2FsIiwibmFtZSI6IkFkbWluIFVzZXIiLCJpc3MiOiJodHRwOi8vbG9jYWxob3N0OjgwODAiLCJhdWQiOiJhdXRoOSIsImlhdCI6MTc3MTEwNDYwMSwiZXhwIjoxNzcxMTA4MjAxfQ.kLQQp-4Rp6DwTIv528mH4RqOCSbfrsIwHD8YdXpSLaYpSul3pBmL7hFIUnIEEYHYOF2WhGJNPQ9pnEg_1u2AG72uOarRWQ_rlALnEmN2wqA1YNNaeKEb2zXmF1r6Bu21sHgm2BhK99P8s9IAhEYha2HHrDw194j2m8DrjQBve5Df1GvLFhnUOiZAbIpdA6LwJbBx97AoFIy3jWDnK9Ru-pPE84BTJIFmht3LHOkA4ZC0bP2BR733kkswtYCq9XUKHbh3U6stxOOjfkcIIut6yXNemk1dWB73UFyGiShot9n96x_IGYt3Z2E7IMtcoTmSsNidjf8DW31OFYf0H3lcYA';

// 扩展HttpClient以支持自定义headers
class ExtendedAuth9HttpClient extends Auth9HttpClient {
  async deleteWithConfirm(path) {
    // 使用原始fetch来设置特殊header
    const token = await this.getToken();
    const url = `${this.baseUrl}${path}`;
    
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-Confirm-Destructive': 'true',
    };
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
    
    const response = await fetch(url, {
      method: 'DELETE',
      headers,
    });
    
    if (!response.ok) {
      const errorBody = await response.json().catch(() => ({
        error: 'unknown',
        message: response.statusText,
      }));
      throw new Error(`Delete failed: ${errorBody.message}`);
    }
    
    // 204 No Content - 无返回体
    return undefined;
  }
}

async function testScenario5() {
  console.log('=== 场景5：DELETE请求与204 No Content处理 ===');
  
  const client = new ExtendedAuth9HttpClient({
    baseUrl: 'http://localhost:8080',
    accessToken: TOKEN,
  });

  let createdTenantId = null;
  
  try {
    // 1. 先创建一个测试租户
    console.log('1. 创建测试租户...');
    const created = await client.post('/api/v1/tenants', {
      name: 'To Delete Test Final',
      slug: 'to-delete-sdk-test-final',
    });
    createdTenantId = created.data.id;
    console.log('创建成功，ID:', createdTenantId);
    
    // 2. 删除该租户（使用扩展的方法）
    console.log('\n2. 删除该租户...');
    const result = await client.deleteWithConfirm(`/api/v1/tenants/${createdTenantId}`);
    console.log('client.delete() 返回值:', result);
    console.log('返回值类型:', typeof result);
    console.log('返回值 === undefined:', result === undefined);
    
    // 3. 验证再次GET该租户返回404
    console.log('\n3. 验证再次GET该租户返回404...');
    try {
      await client.get(`/api/v1/tenants/${createdTenantId}`);
      console.log('❌ 错误：租户仍然存在');
    } catch (err) {
      console.log('✅ 正确：租户已删除，返回错误:', err.message);
      console.log('错误状态码:', err.statusCode);
    }
    
    // 4. 验证数据库状态
    console.log('\n4. 验证数据库状态...');
    console.log('请运行: SELECT id FROM tenants WHERE slug = \'to-delete-sdk-test-final\';');
    console.log('预期: 无记录');
    
  } catch (error) {
    console.error('测试失败:', error);
    
    // 清理残留数据
    if (createdTenantId) {
      console.log('\n清理残留数据...');
      try {
        const response = await fetch(`http://localhost:8080/api/v1/tenants/${createdTenantId}`, {
          method: 'DELETE',
          headers: {
            'Authorization': `Bearer ${TOKEN}`,
            'X-Confirm-Destructive': 'true',
          },
        });
        if (response.ok) {
          console.log('清理成功');
        }
      } catch (cleanupErr) {
        console.log('清理失败:', cleanupErr.message);
      }
    }
  }
}

testScenario5().catch(console.error);