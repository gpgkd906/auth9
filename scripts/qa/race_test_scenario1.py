import asyncio
import aiohttp
import subprocess
import json
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
            body = await resp.json()
            return {"status": status, "success": status == 200, "message": body.get("message", "")}
    except Exception as e:
        return {"status": 0, "success": False, "message": str(e)}

async def test_password_reset_race():
    print("=" * 60)
    print("场景1: 密码重置Token并发使用测试")
    print("=" * 60)
    
    # 使用已知的token (demo2@example.com的token)
    token = "31ad9707-9015-4ac7-adeb-d401b629f037"
    
    print(f"使用token进行并发测试...")
    
    # 并发测试
    concurrency = 50
    async with aiohttp.ClientSession() as session:
        tasks = []
        for i in range(concurrency):
            task = reset_password(session, token, i)
            tasks.append(task)
        
        print(f"发送 {concurrency} 个并发请求...")
        start_time = time.time()
        results = await asyncio.gather(*tasks)
        elapsed = time.time() - start_time
        
    success_count = sum(1 for r in results if r["success"])
    error_count = sum(1 for r in results if r["status"] in [400, 404])
    other_count = len(results) - success_count - error_count
    
    print(f"\n测试结果:")
    print(f"  成功 (200): {success_count}")
    print(f"  失败 (400/404): {error_count}")
    print(f"  其他: {other_count}")
    print(f"  耗时: {elapsed:.2f}秒")
    
    # 验证数据库状态
    result = subprocess.run([
        "bash", "-c",
        "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT used_at IS NOT NULL FROM password_reset_tokens WHERE id = '{}';\"".format(token)
    ], capture_output=True, text=True)
    
    print(f"\n数据库验证 - Token已使用: {result.stdout.strip()}")
    
    # 检查结果
    if success_count > 1:
        print(f"\n🚨 FAIL: 竞态条件漏洞！{success_count}个请求成功")
        return False
    elif success_count == 1:
        print(f"\n✅ PASS: 只有1个请求成功（预期行为）")
        return True
    else:
        print(f"\n⚠️ 警告: 没有成功的请求")
        return False

if __name__ == "__main__":
    success = asyncio.run(test_password_reset_race())
    exit(0 if success else 1)
