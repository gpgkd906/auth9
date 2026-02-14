import asyncio
import subprocess
import json
import time
import base64

def decode_jwt(token):
    """解码JWT token获取payload"""
    try:
        parts = token.split('.')
        if len(parts) != 3:
            return None
        payload = parts[1]
        # 添加padding
        padding = 4 - len(payload) % 4
        if padding != 4:
            payload += '=' * padding
        decoded = base64.b64decode(payload)
        return json.loads(decoded)
    except:
        return None

async def exchange_token(token, tenant_id, service_id, request_id):
    """执行单个gRPC Token Exchange请求"""
    cmd = [
        '.claude/skills/tools/grpcurl-docker.sh',
        '-insecure',
        '-import-path', '/proto',
        '-proto', 'auth9.proto',
        '-H', 'x-api-key: dev-grpc-api-key',
        '-d', f'{{"identity_token": "{token}", "tenant_id": "{tenant_id}", "service_id": "{service_id}"}}',
        'auth9-grpc-tls:50051',
        'auth9.TokenExchange/ExchangeToken'
    ]
    
    try:
        start_time = time.time()
        result = subprocess.run(cmd, capture_output=True, text=True)
        end_time = time.time()
        
        if result.returncode == 0:
            response = json.loads(result.stdout)
            payload = decode_jwt(response.get('accessToken', ''))
            roles = payload.get('roles', []) if payload else []
            permissions = payload.get('permissions', []) if payload else []
            
            return {
                'request_id': request_id,
                'success': True,
                'status': '200',
                'response': response,
                'roles': roles,
                'permissions': permissions,
                'latency': end_time - start_time
            }
        else:
            return {
                'request_id': request_id,
                'success': False,
                'status': 'error',
                'error': result.stderr,
                'latency': end_time - start_time
            }
    except Exception as e:
        return {
            'request_id': request_id,
            'success': False,
            'status': 'exception',
            'error': str(e),
            'latency': 0
        }

async def run_concurrent_test(concurrency=20):
    """运行并发测试"""
    print('=== gRPC Token Exchange 竞态条件测试 ===')
    
    # 生成token
    print('1. 生成Identity Token...')
    result = subprocess.run(['.claude/skills/tools/gen-admin-token.sh'], 
                          capture_output=True, text=True)
    token = result.stdout.strip()
    
    if not token:
        print('无法生成token')
        return
    
    print(f'Token生成成功: {token[:30]}...')
    
    # 获取platform租户ID
    print('2. 获取租户信息...')
    result = subprocess.run([
        'mysql', '-h', '127.0.0.1', '-P', '4000', '-u', 'root', 'auth9', '-N',
        '-e', "SELECT id FROM tenants WHERE slug = 'auth9-platform';"
    ], capture_output=True, text=True)
    
    tenant_id = result.stdout.strip()
    service_id = 'auth9-portal'
    
    print(f'租户ID: {tenant_id}')
    print(f'服务ID: {service_id}')
    
    # 创建并发任务
    print(f'3. 准备 {concurrency} 个并发请求...')
    tasks = []
    for i in range(concurrency):
        task = exchange_token(token, tenant_id, service_id, i+1)
        tasks.append(task)
    
    # 执行并发请求
    print(f'4. 同时发送 {concurrency} 个请求...')
    start_time = time.time()
    results = await asyncio.gather(*tasks)
    end_time = time.time()
    
    # 分析结果
    success_count = sum(1 for r in results if r['success'])
    failure_count = len(results) - success_count
    
    print(f'\n测试结果:')
    print(f'总请求数: {len(results)}')
    print(f'成功数: {success_count}')
    print(f'失败数: {failure_count}')
    print(f'总耗时: {end_time - start_time:.2f}秒')
    
    # 检查权限一致性
    if success_count > 0:
        first_success = next(r for r in results if r['success'])
        expected_roles = first_success['roles']
        expected_permissions = first_success['permissions']
        
        inconsistent = 0
        for result in results:
            if result['success']:
                if result['roles'] != expected_roles or result['permissions'] != expected_permissions:
                    inconsistent += 1
                    print(f'请求 {result["request_id"]}: 权限不一致')
                    print(f'  Roles: {result["roles"]} (预期: {expected_roles})')
                    print(f'  Permissions: {result["permissions"]} (预期: {expected_permissions})')
        
        if inconsistent > 0:
            print(f'\n⚠️  发现 {inconsistent} 个权限不一致的Token')
        else:
            print('\n✅ 所有Token权限一致')
    
    # 显示详细结果
    print('\n详细结果 (前10个):')
    for i, result in enumerate(results[:10]):
        status = '✅' if result['success'] else '❌'
        print(f'请求 {result["request_id"]}: {status} {result["status"]} - 耗时: {result["latency"]:.3f}s')
        if result['success']:
            print(f'  Roles: {result["roles"]}')
    
    # 安全评估
    if success_count == concurrency:
        print('\n✅ 所有请求成功完成')
        print('  需要进一步验证权限一致性')
    elif success_count > 0:
        print(f'\n✅ {success_count} 个请求成功')
    else:
        print('\n❌ 所有请求失败')

if __name__ == '__main__':
    asyncio.run(run_concurrent_test(concurrency=10))