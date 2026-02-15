import requests
import concurrent.futures
import time

TOKEN = "ed77f54774aa64b90a52106539eb79cd8244083d09add93fee58aba2da2f8bd1"
URL = "http://localhost:8080/api/v1/auth/reset-password"

def reset_password(i):
    data = {
        "token": TOKEN,
        "new_password": f"NewPass{i}!"
    }
    try:
        response = requests.post(URL, json=data, timeout=5)
        return response.status_code, response.text
    except Exception as e:
        return 0, str(e)

def main():
    print("Starting concurrent password reset test...")
    
    # 使用线程池并发发送请求
    with concurrent.futures.ThreadPoolExecutor(max_workers=50) as executor:
        futures = [executor.submit(reset_password, i) for i in range(50)]
        
        results = []
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())
    
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
        # 显示成功的响应
        for i, (status, text) in enumerate(results):
            if status == 200:
                print(f"  Request {i}: {text}")
    elif success_count == 1:
        print(f"\n✅ PASS: Only 1 successful reset (expected)")
    else:
        print(f"\n❌ FAIL: No successful resets")
    
    # 显示一些响应示例
    print(f"\nSample responses:")
    for i, (status, text) in enumerate(results[:5]):
        print(f"  Request {i}: Status {status}, Response: {text[:100]}...")

if __name__ == "__main__":
    main()