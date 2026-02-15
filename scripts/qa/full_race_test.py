import requests
import concurrent.futures
import time
import json
import hashlib

def get_reset_token(email):
    """请求密码重置token"""
    # 清理旧邮件
    requests.delete("http://localhost:8025/api/v1/messages")
    time.sleep(1)
    
    # 请求重置
    response = requests.post(
        "http://localhost:8080/api/v1/auth/forgot-password",
        json={"email": email}
    )
    print(f"Forgot password response: {response.status_code} - {response.text}")
    time.sleep(2)
    
    # 获取邮件
    mail_response = requests.get("http://localhost:8025/api/v1/messages")
    messages = mail_response.json().get("messages", [])
    
    if not messages:
        print("No reset email found!")
        return None
    
    # 获取最新邮件
    latest_msg = messages[0]
    msg_id = latest_msg["ID"]
    
    # 获取邮件内容
    msg_response = requests.get(f"http://localhost:8025/api/v1/message/{msg_id}")
    html_content = msg_response.json().get("HTML", "")
    
    # 提取token
    import re
    match = re.search(r'token=([a-f0-9]+)', html_content)
    if match:
        token = match.group(1)
        print(f"Found reset token: {token[:20]}...")
        return token
    else:
        print("Could not extract token from email")
        return None

def reset_password(token, password_suffix):
    """重置密码"""
    data = {
        "token": token,
        "new_password": f"NewPass{password_suffix}!"
    }
    try:
        response = requests.post(
            "http://localhost:8080/api/v1/auth/reset-password",
            json=data,
            timeout=5
        )
        return response.status_code, response.text
    except Exception as e:
        return 0, str(e)

def test_concurrent_reset():
    """测试并发密码重置"""
    email = "testuser@example.com"
    
    print("Step 1: Getting reset token...")
    token = get_reset_token(email)
    if not token:
        print("Failed to get reset token")
        return False
    
    print(f"\nStep 2: Testing token validity...")
    # 先测试单个请求
    status, text = reset_password(token, "test")
    print(f"Single request test: Status {status}, Response: {text[:100]}")
    
    if status != 200:
        print("Token already invalid, trying fresh token...")
        # 再试一次
        token = get_reset_token(email)
        if not token:
            return False
    
    print(f"\nStep 3: Starting concurrent test (50 requests)...")
    
    # 使用线程池并发发送请求
    start_time = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=50) as executor:
        futures = [executor.submit(reset_password, token, i) for i in range(50)]
        
        results = []
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())
    
    elapsed = time.time() - start_time
    print(f"Concurrent test completed in {elapsed:.2f} seconds")
    
    # 分析结果
    success_count = sum(1 for status, _ in results if status == 200)
    error_count = sum(1 for status, _ in results if status == 400 or status == 404)
    other_count = len(results) - success_count - error_count
    
    print(f"\nResults:")
    print(f"  Success (200): {success_count}")
    print(f"  Error (400/404): {error_count}")
    print(f"  Other: {other_count}")
    
    # 检查竞态条件
    if success_count > 1:
        print(f"\n🚨 RACE CONDITION DETECTED: {success_count} successful resets!")
        return False
    elif success_count == 1:
        print(f"\n✅ PASS: Only 1 successful reset (expected)")
        return True
    else:
        print(f"\n❌ FAIL: No successful resets")
        return False

if __name__ == "__main__":
    success = test_concurrent_reset()
    exit(0 if success else 1)