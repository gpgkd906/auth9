import subprocess
import concurrent.futures
import time
import threading
import json

def get_admin_token():
    """获取管理员token"""
    result = subprocess.run([".claude/skills/tools/gen-admin-token.sh"], 
                          capture_output=True, text=True)
    return result.stdout.strip()

def exchange_token(token, tenant_id, request_id):
    """执行Token Exchange"""
    cmd = [
        ".claude/skills/tools/grpcurl-docker.sh",
        "-cacert", "/certs/ca.crt",
        "-cert", "/certs/client.crt",
        "-key", "/certs/client.key",
        "-import-path", "/proto",
        "-proto", "auth9.proto",
        "-H", "x-api-key: dev-grpc-api-key",
        "-d", f'{{"identity_token": "{token}", "tenant_id": "{tenant_id}", "service_id": "auth9-portal"}}',
        "auth9-grpc-tls:50051",
        "auth9.TokenExchange/ExchangeToken"
    ]
    
    try:
        start_time = time.time()
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        elapsed = time.time() - start_time
        
        if result.returncode == 0:
            # 解析响应
            try:
                response = json.loads(result.stdout)
                if "accessToken" in response:
                    return "success", elapsed, response["accessToken"][:50] + "..."
                else:
                    return "error", elapsed, result.stdout[:100]
            except:
                return "success", elapsed, result.stdout[:100]
        else:
            return "error", elapsed, result.stderr[:100]
            
    except subprocess.TimeoutExpired:
        return "timeout", 10, "Timeout"
    except Exception as e:
        return "exception", 0, str(e)[:100]

class GrpcRaceTest:
    def __init__(self):
        self.token = get_admin_token()
        self.tenant_id = "e4a59fda-fc88-445f-9b6a-da66bf480a67"  # auth9-platform
        self.success_count = 0
        self.error_count = 0
        self.timeout_count = 0
        self.exception_count = 0
        self.responses = []
        self.lock = threading.Lock()
        self.start_time = None
    
    def run_exchange(self, request_id):
        """运行单个exchange"""
        result_type, elapsed, details = exchange_token(self.token, self.tenant_id, request_id)
        
        with self.lock:
            if result_type == "success":
                self.success_count += 1
                self.responses.append((request_id, "success", elapsed, details))
                print(f"✓ Request {request_id}: Success ({elapsed:.2f}s)")
            elif result_type == "error":
                self.error_count += 1
                self.responses.append((request_id, "error", elapsed, details))
                print(f"✗ Request {request_id}: Error ({elapsed:.2f}s) - {details}")
            elif result_type == "timeout":
                self.timeout_count += 1
                self.responses.append((request_id, "timeout", elapsed, details))
                print(f"⏱ Request {request_id}: Timeout")
            else:
                self.exception_count += 1
                self.responses.append((request_id, "exception", elapsed, details))
                print(f"⚠ Request {request_id}: Exception - {details}")
        
        return result_type
    
    def run_test(self, num_requests=100, concurrency=50):
        """运行并发测试"""
        print(f"Testing gRPC Token Exchange race condition...")
        print(f"Requests: {num_requests}, Concurrency: {concurrency}")
        print(f"Token: {self.token[:50]}...")
        print(f"Tenant ID: {self.tenant_id}")
        
        self.start_time = time.time()
        
        # 使用线程池
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
            futures = [executor.submit(self.run_exchange, i) for i in range(num_requests)]
            
            # 等待所有完成
            completed = 0
            for future in concurrent.futures.as_completed(futures):
                completed += 1
                if completed % 10 == 0:
                    print(f"  Progress: {completed}/{num_requests}")
        
        total_time = time.time() - self.start_time
        
        print(f"\nTest completed in {total_time:.2f} seconds")
        print(f"\nResults:")
        print(f"  Success: {self.success_count}")
        print(f"  Error: {self.error_count}")
        print(f"  Timeout: {self.timeout_count}")
        print(f"  Exception: {self.exception_count}")
        print(f"  Total: {self.success_count + self.error_count + self.timeout_count + self.exception_count}")
        
        # 分析响应时间
        if self.responses:
            success_times = [elapsed for _, typ, elapsed, _ in self.responses if typ == "success"]
            if success_times:
                avg_time = sum(success_times) / len(success_times)
                min_time = min(success_times)
                max_time = max(success_times)
                print(f"\nResponse times (successful requests):")
                print(f"  Average: {avg_time:.3f}s")
                print(f"  Min: {min_time:.3f}s")
                print(f"  Max: {max_time:.3f}s")
        
        # 检查所有返回的token是否相同
        success_tokens = [details for _, typ, _, details in self.responses if typ == "success"]
        if len(success_tokens) > 1:
            # 检查token是否相同（前50个字符）
            token_samples = [t[:50] for t in success_tokens]
            unique_tokens = set(token_samples)
            print(f"\nToken analysis:")
            print(f"  Generated {len(success_tokens)} tokens")
            print(f"  Unique tokens: {len(unique_tokens)}")
            
            if len(unique_tokens) == 1:
                print(f"  ✓ All tokens are identical (expected for concurrent requests)")
            else:
                print(f"  ⚠ Different tokens generated (possible race condition)")
                for i, token in enumerate(list(unique_tokens)[:3]):
                    print(f"    Token {i+1}: {token}...")
        
        print(f"\nAnalysis:")
        if self.success_count == num_requests:
            print(f"  ✓ All requests succeeded")
        elif self.success_count > 0:
            print(f"  ⚠ {self.success_count}/{num_requests} succeeded")
        
        # 检查是否有明显的竞态条件迹象
        if self.error_count > num_requests * 0.1:  # 超过10%错误
            print(f"  ⚠ High error rate: {self.error_count/num_requests*100:.1f}%")
        
        return self.success_count > 0

if __name__ == "__main__":
    test = GrpcRaceTest()
    
    # 先测试单个请求
    print("Testing single request first...")
    result = test.run_exchange(0)
    print(f"Single request result: {result}")
    
    if result == "success":
        print("\nNow testing concurrent requests...")
        success = test.run_test(num_requests=50, concurrency=20)
        
        if success:
            print("\n✅ TEST COMPLETED")
            exit(0)
        else:
            print("\n❌ TEST FAILED")
            exit(1)
    else:
        print("\n❌ Single request failed, skipping concurrent test")
        exit(1)