import requests
import concurrent.futures
import time
import subprocess
import threading

def get_admin_token():
    """获取管理员token"""
    result = subprocess.run([".claude/skills/tools/gen-admin-token.sh"], 
                          capture_output=True, text=True)
    return result.stdout.strip()

class RaceTest:
    def __init__(self):
        self.token = get_admin_token()
        self.headers = {
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json"
        }
        self.slug = f"race-test-{int(time.time())}"
        self.success_count = 0
        self.conflict_count = 0
        self.db_error_count = 0
        self.other_count = 0
        self.lock = threading.Lock()
    
    def create_tenant(self, suffix):
        """创建租户"""
        url = "http://localhost:8080/api/v1/tenants"
        data = {
            "name": f"Race Tenant {suffix}",
            "slug": self.slug
        }
        
        try:
            response = requests.post(url, headers=self.headers, json=data, timeout=10)
            
            with self.lock:
                if response.status_code == 201:
                    self.success_count += 1
                    print(f"✓ Success {suffix}: Tenant created")
                elif response.status_code == 409:
                    self.conflict_count += 1
                elif response.status_code == 500 and "1062" in response.text:
                    self.db_error_count += 1
                else:
                    self.other_count += 1
                
            return response.status_code, response.text
        except Exception as e:
            with self.lock:
                self.other_count += 1
            return 0, str(e)
    
    def run_test(self, num_requests=20, concurrency=20):
        """运行测试"""
        print(f"Testing tenant slug race condition...")
        print(f"Slug: {self.slug}")
        print(f"Requests: {num_requests}, Concurrency: {concurrency}")
        
        # 确保没有现有租户
        self.cleanup()
        
        print("\nStarting concurrent creation...")
        start_time = time.time()
        
        # 使用线程池
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
            futures = [executor.submit(self.create_tenant, i) for i in range(num_requests)]
            
            # 等待所有完成
            for future in concurrent.futures.as_completed(futures):
                future.result()  # 获取结果（已在create_tenant中处理）
        
        elapsed = time.time() - start_time
        
        print(f"\nTest completed in {elapsed:.2f} seconds")
        print(f"\nResults:")
        print(f"  Success (201): {self.success_count}")
        print(f"  Conflict (409): {self.conflict_count}")
        print(f"  DB Error (500/1062): {self.db_error_count}")
        print(f"  Other: {self.other_count}")
        
        # 验证数据库状态
        self.verify()
        
        # 分析
        print(f"\nAnalysis:")
        if self.success_count == 1:
            print(f"  ✓ Only 1 tenant created (expected)")
        else:
            print(f"  ✗ {self.success_count} tenants created (expected 1)")
        
        if self.db_error_count > 0:
            print(f"  ⚠  {self.db_error_count} database constraint violations")
            print(f"     This indicates race condition at application level")
        
        total_expected = self.success_count + self.conflict_count + self.db_error_count
        if total_expected == num_requests:
            print(f"  ✓ All requests accounted for")
        else:
            print(f"  ✗ Missing {num_requests - total_expected} responses")
        
        # 清理
        self.cleanup()
        
        return self.success_count == 1
    
    def cleanup(self):
        """清理测试租户"""
        # 检查并删除现有租户
        check_url = f"http://localhost:8080/api/v1/tenants?search={self.slug}"
        response = requests.get(check_url, headers=self.headers)
        
        if response.status_code == 200:
            tenants = response.json().get("data", [])
            for tenant in tenants:
                delete_url = f"http://localhost:8080/api/v1/tenants/{tenant['id']}"
                delete_headers = {**self.headers, "X-Confirm-Destructive": "true"}
                requests.delete(delete_url, headers=delete_headers)
                print(f"Deleted existing tenant: {tenant['name']}")
                time.sleep(0.1)
    
    def verify(self):
        """验证数据库状态"""
        check_url = f"http://localhost:8080/api/v1/tenants?search={self.slug}"
        response = requests.get(check_url, headers=self.headers)
        
        if response.status_code == 200:
            tenants = response.json().get("data", [])
            print(f"\nDatabase verification:")
            print(f"  Found {len(tenants)} tenant(s) with slug '{self.slug}':")
            for tenant in tenants:
                print(f"    - {tenant['name']} (ID: {tenant['id']})")
            
            # 检查数据库中的实际数量
            import os
            db_cmd = f"mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e \"SELECT COUNT(*) FROM tenants WHERE slug = '{self.slug}';\""
            result = os.popen(db_cmd).read().strip()
            print(f"  Database count: {result}")
        else:
            print(f"Verification failed: {response.status_code}")

if __name__ == "__main__":
    test = RaceTest()
    success = test.run_test(num_requests=20, concurrency=20)
    
    if success:
        print("\n✅ TEST PASSED: No race condition detected")
        exit(0)
    else:
        print("\n❌ TEST FAILED: Possible race condition")
        exit(1)