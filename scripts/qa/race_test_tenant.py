import asyncio
import aiohttp
import time
import json

async def create_tenant(session, token, slug_suffix, request_id):
    """创建租户"""
    url = "http://localhost:8080/api/v1/tenants"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    
    # 所有请求使用相同的slug
    data = {
        "name": f"Race Tenant {slug_suffix}",
        "slug": "race-test-slug"  # 所有请求使用相同的slug
    }
    
    try:
        start_time = time.time()
        async with session.post(url, headers=headers, json=data) as resp:
            end_time = time.time()
            status = resp.status
            body = await resp.json()
            
            return {
                'request_id': request_id,
                'status': status,
                'success': status == 201,
                'response': body,
                'latency': end_time - start_time
            }
    except Exception as e:
        return {
            'request_id': request_id,
            'status': 0,
            'success': False,
            'error': str(e),
            'latency': 0
        }

async def check_tenant_count(session, token, slug):
    """检查指定slug的租户数量"""
    url = f"http://localhost:8080/api/v1/tenants?search={slug}"
    headers = {"Authorization": f"Bearer {token}"}
    
    async with session.get(url, headers=headers) as resp:
        if resp.status == 200:
            data = await resp.json()
            return data.get('pagination', {}).get('total', 0)
        return 0

async def main():
    print('=== 租户Slug竞态创建测试 ===')
    
    # 获取管理员token
    print('1. 获取管理员token...')
    import subprocess
    result = subprocess.run(['.claude/skills/tools/gen-admin-token.sh'], 
                          capture_output=True, text=True)
    token = result.stdout.strip()
    
    if not token:
        print('无法获取token')
        return
    
    # 首先检查是否已存在相同slug的租户
    print('2. 检查现有租户...')
    async with aiohttp.ClientSession() as session:
        existing_count = await check_tenant_count(session, token, 'race-test-slug')
        if existing_count > 0:
            print(f'发现 {existing_count} 个已存在的race-test-slug租户，需要先清理')
            # 这里可以添加清理逻辑，但为了测试我们使用不同的slug
            test_slug = 'race-test-slug-new'
        else:
            test_slug = 'race-test-slug'
    
    print(f'3. 使用slug: {test_slug}')
    print('4. 准备并发创建请求...')
    
    concurrency = 20
    async with aiohttp.ClientSession() as session:
        # 创建并发任务
        tasks = []
        for i in range(concurrency):
            task = create_tenant(session, token, i+1, i+1)
            tasks.append(task)
        
        # 同时发送所有请求
        print(f'5. 同时发送 {concurrency} 个请求...')
        start_time = time.time()
        results = await asyncio.gather(*tasks)
        end_time = time.time()
        
        # 分析结果
        success_count = sum(1 for r in results if r['success'])
        conflict_count = sum(1 for r in results if r['status'] == 409)
        other_failure = len(results) - success_count - conflict_count
        
        print(f'\n测试结果:')
        print(f'总请求数: {len(results)}')
        print(f'成功创建数 (201): {success_count}')
        print(f'冲突失败数 (409): {conflict_count}')
        print(f'其他失败数: {other_failure}')
        print(f'总耗时: {end_time - start_time:.2f}秒')
        
        # 显示详细结果
        print('\n详细结果 (前10个):')
        for i, result in enumerate(results[:10]):
            status_emoji = '✅' if result['success'] else '❌'
            print(f'请求 {result["request_id"]}: {status_emoji} HTTP {result["status"]} - 耗时: {result["latency"]:.3f}s')
            if not result['success'] and 'response' in result:
                print(f'  错误: {result["response"].get("message", "Unknown")}')
        
        # 检查实际创建的租户数量
        print('\n6. 验证数据库状态...')
        final_count = await check_tenant_count(session, token, test_slug)
        print(f'数据库中 {test_slug} 租户数量: {final_count}')
        
        # 安全评估
        if success_count > 1:
            print(f'\n❌ 检测到竞态条件漏洞！')
            print(f'  成功创建了 {success_count} 个相同slug的租户')
            print(f'  数据库中有 {final_count} 个重复slug的租户')
            print('  攻击者可能利用此漏洞创建重复租户造成混乱')
        elif success_count == 1 and final_count == 1:
            print('\n✅ 安全：只有一个租户创建成功')
            print('  数据库唯一约束正常工作')
        elif success_count == 0 and conflict_count > 0:
            print('\n⚠️  注意：所有请求都返回409冲突')
            print('  可能是slug已被占用，或存在其他问题')
        else:
            print(f'\n⚠️  异常情况：success={success_count}, final={final_count}')

if __name__ == '__main__':
    asyncio.run(main())