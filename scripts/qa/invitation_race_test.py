import requests
import concurrent.futures
import time
import subprocess
import json

def get_admin_token():
    result = subprocess.run([".claude/skills/tools/gen-admin-token.sh"], 
                          capture_output=True, text=True)
    return result.stdout.strip()

def create_invitation(token, email, tenant_id):
    """创建邀请"""
    url = f"http://localhost:8080/api/v1/tenants/{tenant_id}/invitations"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    
    # 获取角色ID
    role_result = subprocess.run([
        "bash", "-c",
        "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT id FROM roles LIMIT 1;\""
    ], capture_output=True, text=True)
    role_id = role_result.stdout.strip()
    
    data = {
        "email": email,
        "tenant_id": tenant_id,
        "role_ids": [role_id]
    }
    
    try:
        response = requests.post(url, headers=headers, json=data, timeout=10)
        return response.status_code, response.text
    except Exception as e:
        return 0, str(e)

def accept_invitation(user_token, invite_token):
    """接受邀请"""
    url = "http://localhost:8080/api/v1/invitations/accept"
    headers = {
        "Authorization": f"Bearer {user_token}",
        "Content-Type": "application/json"
    }
    data = {"token": invite_token}
    
    try:
        response = requests.post(url, headers=headers, json=data, timeout=10)
        return response.status_code, response.text
    except Exception as e:
        return 0, str(e)

def test_invitation_race():
    """测试邀请接受竞态条件"""
    print("=" * 60)
    print("场景2: 邀请接受竞态条件测试")
    print("=" * 60)
    
    # 获取admin token
    admin_token = get_admin_token()
    print(f"Admin token obtained")
    
    # 获取租户ID
    tenant_result = subprocess.run([
        "bash", "-c",
        "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT id FROM tenants WHERE slug = 'auth9-platform' LIMIT 1;\""
    ], capture_output=True, text=True)
    tenant_id = tenant_result.stdout.strip()
    print(f"Tenant ID: {tenant_id}")
    
    # 创建测试邀请
    invite_email = f"race-invite-{int(time.time())}@test.com"
    print(f"Creating invitation for {invite_email}...")
    
    status, text = create_invitation(admin_token, invite_email, tenant_id)
    print(f"Create invitation response: {status}")
    
    if status != 200 and status != 201:
        print(f"Failed to create invitation: {text}")
        return False
    
    # 获取邀请token
    import re
    match = re.search(r'"token":"([^"]+)"', text)
    if not match:
        # 尝试从数据库获取
        result = subprocess.run([
            "bash", "-c",
            "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT token FROM invitations WHERE email = '{}' ORDER BY created_at DESC LIMIT 1;\"".format(invite_email)
        ], capture_output=True, text=True)
        invite_token = result.stdout.strip()
    else:
        invite_token = match.group(1)
    
    print(f"Invite token: {invite_token[:30]}...")
    
    # 准备用户token（用于接受邀请）
    # 由于需要用户先登录，我们创建一个测试用户
    print("Creating test user...")
    user_email = f"race-user-{int(time.time())}@test.com"
    register_resp = requests.post(
        "http://localhost:8080/api/v1/auth/register",
        json={
            "email": user_email,
            "password": "TestPass123!",
            "display_name": "Race Test User"
        },
        timeout=10
    )
    print(f"User registration: {register_resp.status_code}")
    
    # 登录获取user token
    login_resp = requests.post(
        "http://localhost:8080/api/v1/auth/login",
        json={
            "email": user_email,
            "password": "TestPass123!"
        },
        timeout=10
    )
    if login_resp.status_code != 200:
        print(f"Login failed: {login_resp.text}")
        # 尝试使用admin token作为user token
        user_token = admin_token
    else:
        user_token = login_resp.json().get("access_token", admin_token)
    
    print(f"\nTesting concurrent invitation acceptance (20 requests)...")
    
    # 并发接受邀请
    start_time = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=20) as executor:
        futures = [executor.submit(accept_invitation, user_token, invite_token) for _ in range(20)]
        
        results = []
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())
    
    elapsed = time.time() - start_time
    print(f"Concurrent test completed in {elapsed:.2f} seconds")
    
    # 分析结果
    success_count = sum(1 for status, _ in results if status == 200)
    conflict_count = sum(1 for status, _ in results if status == 400)
    other_count = len(results) - success_count - conflict_count
    
    print(f"\nResults:")
    print(f"  Success (200): {success_count}")
    print(f"  Conflict (400): {conflict_count}")
    print(f"  Other: {other_count}")
    
    # 显示其他响应的示例
    other_responses = [(s, t) for s, t in results if s not in [200, 400]]
    if other_responses:
        print(f"\nOther response samples:")
        for status, text in other_responses[:3]:
            print(f"  Status {status}: {text[:100]}...")
    
    # 验证数据库状态
    user_result = subprocess.run([
        "bash", "-c",
        "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT id FROM users WHERE email = '{}';\"".format(user_email)
    ], capture_output=True, text=True)
    user_id = user_result.stdout.strip()
    
    tenant_result = subprocess.run([
        "bash", "-c",
        "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT id FROM tenants WHERE slug = 'auth9-platform';\""
    ], capture_output=True, text=True)
    tenant_id = tenant_result.stdout.strip()
    
    if user_id and tenant_id:
        count_result = subprocess.run([
            "bash", "-c",
            "mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \"SELECT COUNT(*) FROM tenant_users WHERE user_id = '{}' AND tenant_id = '{}';\"".format(user_id, tenant_id)
        ], capture_output=True, text=True)
        tenant_user_count = int(count_result.stdout.strip())
        print(f"\nDatabase verification:")
        print(f"  tenant_users count for this user+tenant: {tenant_user_count}")
    
    # 检查结果
    if success_count > 1:
        print(f"\n🚨 FAIL: 竞态条件漏洞！{success_count}个请求成功")
        return False
    elif success_count == 1:
        print(f"\n✅ PASS: 只有1个请求成功（预期行为）")
        return True
    else:
        print(f"\n⚠️ 结果异常: {success_count}个成功")
        return False

if __name__ == "__main__":
    success = test_invitation_race()
    exit(0 if success else 1)
