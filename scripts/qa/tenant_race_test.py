import requests
import concurrent.futures
import time
import json

def get_admin_token():
    """获取管理员token"""
    import subprocess
    result = subprocess.run([".claude/skills/tools/gen-admin-token.sh"], 
                          capture_output=True, text=True)
    return result.stdout.strip()

def create_tenant(token, suffix):
    """创建租户"""
    url = "http://localhost:8080/api/v1/tenants"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    data = {
        "name": f"Race Tenant {suffix}",
        "slug": "race-test-slug"  # 所有请求使用相同的slug
    }
    
    try:
        response = requests.post(url, headers=headers, json=data, timeout=10)
        return response.status_code, response.text
    except Exception as e:
        return 0, str(e)

def test_tenant_slug_race():
    """测试租户slug竞态创建"""
    print("Testing tenant slug race condition...")
    
    # 获取管理员token
    token = get_admin_token()
    print(f"Admin token obtained: {token[:50]}...")
    
    # 先检查是否已存在同slug的租户
    check_url = "http://localhost:8080/api/v1/tenants?search=race-test-slug"
    check_headers = {"Authorization": f"Bearer {token}"}
    check_response = requests.get(check_url, headers=check_headers)
    
    if check_response.status_code == 200:
        existing = check_response.json().get("data", [])
        if existing:
            print(f"Found existing tenant with slug 'race-test-slug': {existing[0]['name']}")
            print("Deleting existing tenant...")
            # 删除现有租户
            tenant_id = existing[0]["id"]
            delete_url = f"http://localhost:8080/api/v1/tenants/{tenant_id}"
            delete_response = requests.delete(delete_url, headers=check_headers)
            print(f"Delete response: {delete_response.status_code}")
            time.sleep(1)
    
    print("\nStarting concurrent tenant creation (20 requests)...")
    
    # 使用线程池并发发送请求
    start_time = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=20) as executor:
        futures = [executor.submit(create_tenant, token, i) for i in range(20)]
        
        results = []
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())
    
    elapsed = time.time() - start_time
    print(f"Concurrent test completed in {elapsed:.2f} seconds")
    
    # 分析结果
    success_count = sum(1 for status, _ in results if status == 201)
    conflict_count = sum(1 for status, _ in results if status == 409)
    other_count = len(results) - success_count - conflict_count
    
    print(f"\nResults:")
    print(f"  Success (201): {success_count}")
    print(f"  Conflict (409): {conflict_count}")
    print(f"  Other: {other_count}")
    
    # 显示响应示例
    print(f"\nSample responses:")
    for i, (status, text) in enumerate(results[:5]):
        print(f"  Request {i}: Status {status}, Response: {text[:100]}...")
    
    # 检查实际创建的租户数量
    print(f"\nVerifying database state...")
    verify_response = requests.get(check_url, headers=check_headers)
    if verify_response.status_code == 200:
        tenants = verify_response.json().get("data", [])
        print(f"Found {len(tenants)} tenant(s) with slug 'race-test-slug':")
        for tenant in tenants:
            print(f"  - {tenant['name']} (ID: {tenant['id']})")
    
    # 检查竞态条件
    if success_count > 1:
        print(f"\n🚨 RACE CONDITION DETECTED: {success_count} successful creations!")
        return False
    elif success_count == 1 and conflict_count == 19:
        print(f"\n✅ PASS: Only 1 successful creation, 19 conflicts (expected)")
        return True
    else:
        print(f"\n❌ UNEXPECTED: {success_count} successes, {conflict_count} conflicts")
        return False

if __name__ == "__main__":
    success = test_tenant_slug_race()
    exit(0 if success else 1)