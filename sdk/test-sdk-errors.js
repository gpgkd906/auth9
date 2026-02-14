const { Auth9HttpClient, NotFoundError, ValidationError, UnauthorizedError } = require('./packages/core/dist/index.cjs');

const TOKEN = process.env.AUTH9_API_KEY;
const TENANT_ID = '0df463ad-10a2-4589-8708-0b56dba70161';

async function testErrors() {
  console.log('=== 场景8：错误处理 - SDK ===');
  
  try {
    // 测试认证失败
    console.log('1. 测试认证失败...');
    const invalidClient = new Auth9HttpClient({
      baseUrl: 'http://localhost:8080',
      accessToken: 'invalid-token',
    });
    
    try {
      await invalidClient.get(`/api/v1/tenants/${TENANT_ID}/actions`);
      console.log('❌ 预期认证失败但请求成功');
    } catch (error) {
      if (error instanceof UnauthorizedError) {
        console.log('✅ 认证失败测试通过:', error.message);
      } else {
        console.log('❌ 认证失败测试: 错误类型不正确', error.constructor.name, error.message);
      }
    }

    // 测试 404
    console.log('2. 测试404错误...');
    const validClient = new Auth9HttpClient({
      baseUrl: 'http://localhost:8080',
      accessToken: TOKEN,
    });
    
    try {
      await validClient.get(`/api/v1/tenants/${TENANT_ID}/actions/00000000-0000-0000-0000-000000000000`);
      console.log('❌ 预期404但请求成功');
    } catch (error) {
      if (error instanceof NotFoundError) {
        console.log('✅ 404测试通过:', error.message);
      } else {
        console.log('❌ 404测试: 错误类型不正确', error.constructor.name, error.message);
      }
    }

    // 测试验证错误
    console.log('3. 测试验证错误...');
    try {
      await validClient.post(`/api/v1/tenants/${TENANT_ID}/actions`, {
        name: '',
        trigger_id: 'invalid-trigger',
        script: '',
      });
      console.log('❌ 预期验证错误但请求成功');
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('✅ 验证错误测试通过:', error.message);
      } else {
        console.log('❌ 验证错误测试: 错误类型不正确', error.constructor.name, error.message);
      }
    }

    console.log('✅ 场景8测试完成');
    
  } catch (error) {
    console.error('❌ 场景8测试失败:', error.message);
    console.error(error);
  }
}

testErrors();
