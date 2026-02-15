#!/usr/bin/env python3
import base64
import os
import subprocess
import sys

print("加密密钥强度与管理测试")
print("=" * 60)

# 1. 检查代码中是否硬编码密钥
print("1. 检查代码中是否硬编码密钥")
print("-" * 40)

commands = [
    ("检查Rust代码", "grep -r 'SETTINGS_ENCRYPTION_KEY\|encryption_key' auth9-core/src/ --include='*.rs' | grep -v 'env\|config\|test'"),
    ("检查配置文件", "grep -r 'SETTINGS_ENCRYPTION_KEY' docker-compose*.yml .env* Dockerfile* 2>/dev/null || true"),
    ("检查Git历史", "git log -p --all -S 'SETTINGS_ENCRYPTION_KEY' -- '*.toml' '*.yaml' '*.yml' '*.env' 2>/dev/null | head -50"),
]

for desc, cmd in commands:
    print(f"\n{desc}:")
    print(f"命令: {cmd}")
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.stdout.strip():
        print(f"❌ 发现潜在问题:")
        print(result.stdout[:500])
    else:
        print("✅ 未发现硬编码密钥")
print()

# 2. 检查当前环境中的密钥
print("2. 检查当前环境中的密钥")
print("-" * 40)

# 从Docker容器获取密钥
result = subprocess.run(
    ["docker", "exec", "auth9-core", "env", "|", "grep", "SETTINGS_ENCRYPTION_KEY"],
    capture_output=True,
    text=True
)

if result.returncode == 0 and result.stdout:
    key_line = result.stdout.strip()
    if "=" in key_line:
        key_b64 = key_line.split("=", 1)[1]
        print(f"环境变量中的密钥: {key_b64[:20]}...")
        
        try:
            key_bytes = base64.b64decode(key_b64)
            print(f"密钥长度: {len(key_bytes)} bytes ({len(key_bytes)*8} bits)")
            
            if len(key_bytes) == 32:
                print("✅ 密钥长度正确 (256 bits)")
                
                # 检查密钥强度（简单的熵检查）
                unique_bytes = len(set(key_bytes))
                print(f"唯一字节数: {unique_bytes}/256")
                if unique_bytes > 20:  # 简单阈值
                    print("✅ 密钥看起来有足够的随机性")
                else:
                    print("⚠️  密钥可能随机性不足")
                    
            else:
                print(f"❌ 密钥长度错误: 需要32字节，实际{len(key_bytes)}字节")
                
        except Exception as e:
            print(f"❌ 密钥解码错误: {e}")
    else:
        print("❌ 未找到有效的密钥环境变量")
else:
    print("❌ 未设置SETTINGS_ENCRYPTION_KEY环境变量")
print()

# 3. 测试弱密钥
print("3. 测试弱密钥处理")
print("-" * 40)

test_keys = [
    ("空密钥", ""),
    ("短密钥", "dG9vLXNob3J0"),  # "too-short" in base64
    ("全零密钥", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),  # 32个A的base64
    ("简单模式密钥", "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI="),  # 简单模式
]

for desc, key_b64 in test_keys:
    print(f"\n测试 {desc}: {key_b64[:20]}...")
    
    # 这里应该测试应用是否拒绝弱密钥
    # 在实际测试中，应该启动应用并验证
    if desc == "空密钥" and not key_b64:
        print("预期: 应用启动失败或加密功能禁用")
    elif desc == "短密钥":
        print("预期: 密钥长度验证失败")
    elif desc == "全零密钥":
        print("预期: 应用应警告弱密钥")
    else:
        print("预期: 应用应验证密钥强度")
print()

# 4. 检查密钥生成建议
print("4. 密钥生成建议")
print("-" * 40)

print("推荐生成256位AES密钥的方法:")
print("1. openssl rand -base64 32")
print("2. Python: secrets.token_bytes(32)")
print("3. 使用密钥管理系统 (KMS, Vault, etc.)")
print()

print("密钥管理最佳实践:")
print("✅ 密钥存储在环境变量或密钥管理系统中")
print("✅ 不在代码或配置文件中硬编码")
print("✅ 定期轮换密钥")
print("✅ 使用足够的熵生成密钥")
print("✅ 验证密钥长度和格式")
print()

# 5. 检查密钥轮转机制
print("5. 检查密钥轮转机制")
print("-" * 40)

print("当前实现检查:")
print("1. 代码中是否有密钥轮转支持?")
print("   - 需要检查是否支持多密钥解密")
print("2. 数据迁移策略?")
print("   - 旧数据使用旧密钥解密")
print("   - 新数据使用新密钥加密")
print("3. 密钥版本管理?")
print()

print("=" * 60)
print("测试总结:")
print("需要验证:")
print("1. 应用启动时验证密钥长度")
print("2. 弱密钥被拒绝")
print("3. 密钥不硬编码在代码中")
print("4. Git历史无密钥泄露")
print("5. 有密钥轮转计划")