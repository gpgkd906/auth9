const { Auth9HttpClient } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const SERVICE_ID = '040502d5-e073-4ba2-ae21-4ca8069f0415';

const client = new Auth9HttpClient({
  baseUrl: 'http://localhost:8080',
  accessToken: TOKEN,
});

async function testAction() {
  console.log('=== 场景5：SDK - 测试Action（Test Endpoint） ===');
  
  try {
    // 创建 Action
    console.log('创建测试Action...');
    const action = await client.post(`/api/v1/services/${SERVICE_ID}/actions`, {
      name: 'Test Action',
      trigger_id: 'post-login',
      script: `
        if (context.user.email.endsWith("@blocked.com")) {
          throw new Error("Blocked domain");
        }
        context.claims = context.claims || {};
        context.claims.tested = true;
        context;
      `,
    });
    const actionId = action.data.id;
    console.log('Action ID:', actionId);

    // 测试：允许的邮箱域
    console.log('测试允许的邮箱域...');
    const successResult = await client.post(`/api/v1/services/${SERVICE_ID}/actions/${actionId}/test`, {
      context: {
        user: {
          id: 'test-user-id',
          email: 'user@allowed.com',
          mfa_enabled: false,
        },
        tenant: {
          id: SERVICE_ID,
          slug: 'test-tenant',
          name: 'Test Tenant',
        },
        request: {
          ip: '1.2.3.4',
          user_agent: 'Mozilla/5.0',
          timestamp: new Date().toISOString(),
        },
      }
    });
    console.log('Success test:', successResult.data?.success); // true
    console.log('Modified claims:', successResult.data?.modified_context?.claims);

    // 测试：阻止的邮箱域
    console.log('测试阻止的邮箱域...');
    const failResult = await client.post(`/api/v1/services/${SERVICE_ID}/actions/${actionId}/test`, {
      context: {
        user: {
          id: 'test-user-id',
          email: 'user@blocked.com',
          mfa_enabled: false,
        },
        tenant: {
          id: SERVICE_ID,
          slug: 'test-tenant',
          name: 'Test Tenant',
        },
        request: {
          ip: '1.2.3.4',
          timestamp: new Date().toISOString(),
        },
      }
    });
    console.log('Fail test:', failResult.data?.success); // false
    console.log('Error:', failResult.data?.error_message); // "Blocked domain"

    // 验证结果
    if (successResult.data?.success === true && 
        successResult.data?.modifiedContext?.claims?.tested === true &&
        failResult.data?.success === false &&
        failResult.data?.errorMessage?.includes('Blocked domain')) {
      console.log('✅ 场景5测试通过');
    } else {
      console.log('❌ 场景5测试失败: 结果不符合预期');
      console.log('成功测试结果:', JSON.stringify(successResult, null, 2));
      console.log('失败测试结果:', JSON.stringify(failResult, null, 2));
    }

    // 清理
    console.log('清理测试数据...');
    await client.delete(`/api/v1/services/${SERVICE_ID}/actions/${actionId}`);
    
  } catch (error) {
    console.error('❌ 场景5测试失败:', error.message);
    console.error(error);
  }
}

testAction();
