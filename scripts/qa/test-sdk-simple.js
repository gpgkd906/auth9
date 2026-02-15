// 简单的SDK测试模拟
const fetch = require('node-fetch');

class SimpleAuth9Client {
  constructor(config) {
    this.baseUrl = config.baseUrl;
    this.apiKey = config.apiKey;
    this.tenantId = config.tenantId;
  }

  async request(method, path, body = null) {
    const url = `${this.baseUrl}${path}`;
    const headers = {
      'Authorization': `Bearer ${this.apiKey}`,
      'Content-Type': 'application/json',
    };

    const options = {
      method,
      headers,
    };

    if (body) {
      options.body = JSON.stringify(body);
    }

    const response = await fetch(url, options);
    const data = await response.json();

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${data.message || 'Unknown error'}`);
    }

    return data;
  }

  get actions() {
    return {
      create: async (data) => {
        const result = await this.request('POST', `/api/v1/tenants/${this.tenantId}/actions`, data);
        return result.data;
      },
      list: async (triggerId) => {
        const path = triggerId 
          ? `/api/v1/tenants/${this.tenantId}/actions?trigger_id=${triggerId}`
          : `/api/v1/tenants/${this.tenantId}/actions`;
        const result = await this.request('GET', path);
        return result.data;
      },
      get: async (id) => {
        const result = await this.request('GET', `/api/v1/tenants/${this.tenantId}/actions/${id}`);
        return result.data;
      },
      update: async (id, data) => {
        const result = await this.request('PATCH', `/api/v1/tenants/${this.tenantId}/actions/${id}`, data);
        return result.data;
      },
      delete: async (id) => {
        await this.request('DELETE', `/api/v1/tenants/${this.tenantId}/actions/${id}`);
        return true;
      },
    };
  }
}

async function testCRUD() {
  const TOKEN = process.env.AUTH9_API_KEY;
  const TENANT_ID = process.env.TENANT_ID;

  if (!TOKEN || !TENANT_ID) {
    console.error('Please set AUTH9_API_KEY and TENANT_ID environment variables');
    process.exit(1);
  }

  const client = new SimpleAuth9Client({
    baseUrl: 'http://localhost:8080',
    apiKey: TOKEN,
    tenantId: TENANT_ID,
  });

  try {
    // 1. 创建 Action
    console.log('Creating action...');
    const action = await client.actions.create({
      name: 'SDK Test Action',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.sdk_test = true; context;',
      enabled: true,
    });
    console.log('Created:', action.id);

    // 2. 获取列表
    console.log('Getting actions list...');
    const actions = await client.actions.list();
    console.log('Total actions:', actions.length);

    // 3. 获取单个
    console.log('Getting single action...');
    const retrieved = await client.actions.get(action.id);
    console.log('Retrieved:', retrieved.name);

    // 4. 更新
    console.log('Updating action...');
    const updated = await client.actions.update(action.id, {
      description: 'Updated via SDK',
    });
    console.log('Updated description:', updated.description);

    // 5. 删除
    console.log('Deleting action...');
    await client.actions.delete(action.id);
    console.log('Deleted successfully');

    console.log('✅ SDK CRUD test passed');
    return true;
  } catch (error) {
    console.error('Error:', error.message);
    console.log('❌ SDK CRUD test failed');
    return false;
  }
}

testCRUD().then(success => {
  process.exit(success ? 0 : 1);
});