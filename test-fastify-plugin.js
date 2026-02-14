// 测试Fastify插件
import { auth9Plugin } from "./sdk/packages/node/dist/middleware/fastify.js";

// 创建模拟的Fastify实例
class MockFastify {
  constructor() {
    this.decorateRequestCalls = [];
    this.hooks = {};
  }
  
  decorateRequest(name, value) {
    this.decorateRequestCalls.push({ name, value });
  }
  
  addHook(name, handler) {
    this.hooks[name] = handler;
  }
}

async function testFastifyPlugin() {
  console.log("=== 场景2：Fastify Plugin测试 ===\n");
  
  // 创建模拟Fastify实例
  const fastify = new MockFastify();
  
  // 注册插件
  await auth9Plugin(fastify, {
    domain: "http://localhost:8080",
    audience: "my-service",
  });
  
  console.log("1. 检查插件注册:");
  console.log(`   decorateRequest调用: ${fastify.decorateRequestCalls.length}次`);
  console.log(`   注册的hook: ${Object.keys(fastify.hooks).join(', ')}`);
  
  if (fastify.decorateRequestCalls.length > 0) {
    const call = fastify.decorateRequestCalls[0];
    console.log(`   装饰的request属性: ${call.name}, 初始值: ${call.value}`);
  }
  
  // 测试hook逻辑
  if (fastify.hooks.onRequest) {
    console.log("\n2. 测试onRequest hook逻辑:");
    
    // 测试无Token的情况
    console.log("   a) 无Token:");
    const request1 = { headers: {}, auth9: undefined };
    const reply1 = { code: () => ({ send: () => {} }) };
    await fastify.hooks.onRequest(request1, reply1);
    console.log(`     request.auth9: ${request1.auth9 === undefined ? 'undefined ✅' : '有值 ❌'}`);
    
    // 测试无效Token的情况
    console.log("   b) 无效Token:");
    const request2 = { headers: { authorization: "Bearer invalid_token" }, auth9: undefined };
    const reply2 = { code: () => ({ send: () => {} }) };
    await fastify.hooks.onRequest(request2, reply2);
    console.log(`     request.auth9: ${request2.auth9 === undefined ? 'undefined ✅' : '有值 ❌'}`);
    
    // 测试有效Token的情况（需要模拟TokenVerifier）
    console.log("   c) 有效Token:");
    console.log("     注意：需要真实的TokenVerifier验证，这里只测试逻辑结构");
  }
  
  console.log("\n3. 测试Auth9FastifyAuth接口方法:");
  console.log("   根据源代码，auth9对象应包含以下方法:");
  console.log("   - hasPermission(permission)");
  console.log("   - hasRole(role)");
  console.log("   - hasAnyPermission(permissions[])");
  console.log("   - hasAllPermissions(permissions[])");
  
  console.log("\n=== 测试总结 ===");
  console.log("Fastify插件基本结构测试完成。");
  console.log("完整功能测试需要真实的TokenVerifier和Fastify环境。");
}

testFastifyPlugin().catch(console.error);