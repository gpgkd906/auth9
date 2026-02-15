import { Auth9HttpClient } from './packages/core/dist/index.js';

async function testScenario4() {
  console.log('=== 场景4：异步Token Provider ===');
  
  let callCount = 0;
  let tokenSequence = ['token-1', 'token-2', 'token-3'];
  
  const client = new Auth9HttpClient({
    baseUrl: 'http://localhost:8080',
    accessToken: async () => {
      callCount++;
      const token = tokenSequence.shift() || 'default-token';
      console.log(`Token函数第${callCount}次调用，返回: ${token}`);
      return token;
    },
  });

  // 模拟fetch来验证token使用
  const originalFetch = globalThis.fetch;
  let capturedHeaders = [];
  
  globalThis.fetch = async (url, init) => {
    capturedHeaders.push(init.headers);
    
    // 模拟成功响应
    return {
      ok: true,
      status: 200,
      json: async () => ({ data: [] }),
    };
  };

  try {
    console.log('\n1. 发送多个请求验证每次调用token函数:');
    await client.get('/api/v1/tenants');
    await client.get('/api/v1/users');
    await client.get('/api/v1/services');
    
    console.log(`\ncallCount = ${callCount} (预期: 3)`);
    console.log('每次请求的Authorization header:');
    capturedHeaders.forEach((headers, index) => {
      console.log(`  请求${index + 1}: ${headers.Authorization}`);
    });
    
    // 2. 验证支持直接string token
    console.log('\n2. 验证支持直接string token:');
    const staticClient = new Auth9HttpClient({
      baseUrl: 'http://localhost:8080',
      accessToken: 'static-token',
    });
    
    capturedHeaders = [];
    await staticClient.get('/api/v1/tenants');
    console.log('静态token请求header:', capturedHeaders[0].Authorization);
    
    // 3. 验证支持同步函数返回token
    console.log('\n3. 验证支持同步函数返回token:');
    const syncClient = new Auth9HttpClient({
      baseUrl: 'http://localhost:8080',
      accessToken: () => 'sync-token',
    });
    
    capturedHeaders = [];
    await syncClient.get('/api/v1/tenants');
    console.log('同步函数token请求header:', capturedHeaders[0].Authorization);
    
  } finally {
    // 恢复原始fetch
    globalThis.fetch = originalFetch;
  }
}

testScenario4().catch(console.error);