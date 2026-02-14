import asyncio
import aiohttp
import time

async def reset_password(session, token, password_suffix):
    url = "http://localhost:8080/api/v1/auth/reset-password"
    data = {
        "token": token,
        "new_password": f"NewPass{password_suffix}!"
    }
    
    try:
        async with session.post(url, json=data) as resp:
            status = resp.status
            if status == 200:
                body = await resp.json()
                return {"status": status, "success": True, "message": body.get("message", "")}
            else:
                body = await resp.json()
                return {"status": status, "success": False, "message": body.get("message", "")}
    except Exception as e:
        return {"status": 0, "success": False, "message": str(e)}

async def main():
    # 首先获取一个新的重置token
    print("1. 获取新的密码重置token...")
    
    # 发送密码重置请求
    async with aiohttp.ClientSession() as session:
        forgot_url = "http://localhost:8080/api/v1/auth/forgot-password"
        forgot_data = {"email": "normaluser@test.com"}
        
        async with session.post(forgot_url, json=forgot_data) as resp:
            if resp.status != 200:
                print(f"密码重置请求失败: {resp.status}")
                return
    
    print("2. 等待邮件发送...")
    await asyncio.sleep(2)  # 等待邮件发送
    
    # 这里需要从mailpit获取token，但为了简化，我们假设有一个新token
    # 在实际测试中，需要从mailpit API获取token
    print("3. 注意：需要从mailpit获取实际token，这里使用示例token")
    print("4. 开始并发测试...")
    
    # 模拟并发请求
    token = "test-token-needs-to-be-replaced"
    concurrency = 20
    
    async with aiohttp.ClientSession() as session:
        tasks = []
        for i in range(concurrency):
            task = reset_password(session, token, i)
            tasks.append(task)
        
        # 同时发送所有请求
        print(f"同时发送 {concurrency} 个请求...")
        start_time = time.time()
        results = await asyncio.gather(*tasks)
        end_time = time.time()
        
        success_count = sum(1 for r in results if r["success"])
        failure_count = len(results) - success_count
        
        print(f"\n测试结果:")
        print(f"总请求数: {len(results)}")
        print(f"成功数: {success_count}")
        print(f"失败数: {failure_count}")
        print(f"总耗时: {end_time - start_time:.2f}秒")
        
        if success_count > 1:
            print("❌ 检测到竞态条件：多个密码重置请求成功！")
        else:
            print("✅ 安全：只有一个密码重置请求成功（或无成功）")

if __name__ == "__main__":
    asyncio.run(main())