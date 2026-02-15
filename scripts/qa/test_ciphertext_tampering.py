#!/usr/bin/env python3
import base64
import json
import sys

print("密文篡改与认证标签验证测试")
print("=" * 60)

# 从数据库获取当前的加密密码
import subprocess
result = subprocess.run(
    ["mysql", "-h", "127.0.0.1", "-P", "4000", "-u", "root", "auth9", 
     "-e", "SELECT JSON_EXTRACT(value, '$.password') FROM system_settings WHERE setting_key = 'provider'"],
    capture_output=True,
    text=True
)

if result.returncode != 0:
    print("❌ 无法从数据库读取加密密码")
    sys.exit(1)

lines = result.stdout.strip().split('\n')
if len(lines) < 2:
    print("❌ 未找到加密密码")
    sys.exit(1)

encrypted_password = lines[1].strip('"')
print(f"原始加密密码: {encrypted_password}")
print()

# 解析密文
try:
    nonce_b64, ciphertext_b64 = encrypted_password.split(':')
    print(f"Nonce (base64): {nonce_b64}")
    print(f"密文 (base64): {ciphertext_b64[:30]}...")
    print()
    
    # 解码
    nonce = base64.b64decode(nonce_b64)
    ciphertext = base64.b64decode(ciphertext_b64)
    
    print(f"Nonce长度: {len(nonce)} bytes")
    print(f"密文长度: {len(ciphertext)} bytes")
    print(f"GCM tag长度: 16 bytes (标准)")
    print(f"实际数据长度: {len(ciphertext) - 16} bytes")
    print()
    
except Exception as e:
    print(f"❌ 密文解析错误: {e}")
    sys.exit(1)

# 测试1: bit-flip攻击
print("测试1: Bit-flip攻击")
print("-" * 40)

tampered_ciphertext = bytearray(ciphertext)
if len(tampered_ciphertext) > 0:
    # 修改第一个字节的一个bit
    tampered_ciphertext[0] ^= 0x01
    tampered_b64 = base64.b64encode(bytes(tampered_ciphertext)).decode()
    tampered_encrypted = f"{nonce_b64}:{tampered_b64}"
    
    print(f"原始密文第一个字节: 0x{ciphertext[0]:02x}")
    print(f"篡改后第一个字节: 0x{tampered_ciphertext[0]:02x}")
    print(f"篡改后的加密值: {tampered_encrypted[:50]}...")
    print("预期: 解密失败 (GCM认证标签验证失败)")
    print()

# 测试2: 截断密文（移除GCM tag）
print("测试2: 截断密文（移除GCM tag）")
print("-" * 40)

if len(ciphertext) > 16:
    truncated_ciphertext = ciphertext[:-16]  # 移除最后16字节的tag
    truncated_b64 = base64.b64encode(truncated_ciphertext).decode()
    truncated_encrypted = f"{nonce_b64}:{truncated_b64}"
    
    print(f"原始密文长度: {len(ciphertext)} bytes")
    print(f"截断后长度: {len(truncated_ciphertext)} bytes")
    print(f"截断后的加密值: {truncated_encrypted[:50]}...")
    print("预期: 解密失败 (缺少GCM认证标签)")
    print()

# 测试3: 修改nonce
print("测试3: 修改Nonce")
print("-" * 40)

tampered_nonce = bytearray(nonce)
if len(tampered_nonce) > 0:
    tampered_nonce[0] ^= 0x01
    tampered_nonce_b64 = base64.b64encode(bytes(tampered_nonce)).decode()
    tampered_nonce_encrypted = f"{tampered_nonce_b64}:{ciphertext_b64}"
    
    print(f"原始Nonce第一个字节: 0x{nonce[0]:02x}")
    print(f"篡改后Nonce第一个字节: 0x{tampered_nonce[0]:02x}")
    print(f"篡改Nonce后的加密值: {tampered_nonce_encrypted[:50]}...")
    print("预期: 解密失败 (Nonce不匹配)")
    print()

# 测试4: 完全替换密文
print("测试4: 完全替换密文")
print("-" * 40)

fake_ciphertext = b"A" * len(ciphertext)  # 伪造相同长度的密文
fake_b64 = base64.b64encode(fake_ciphertext).decode()
fake_encrypted = f"{nonce_b64}:{fake_b64}"

print(f"伪造密文长度: {len(fake_ciphertext)} bytes")
print(f"伪造的加密值: {fake_encrypted[:50]}...")
print("预期: 解密失败 (无效的GCM认证标签)")
print()

print("=" * 60)
print("测试总结:")
print("1. GCM模式提供认证加密，任何密文修改都应导致解密失败")
print("2. 认证标签验证确保密文完整性")
print("3. 测试需要实际尝试解密篡改后的密文来验证")
print()
print("✅ 测试场景定义完成")
print("⚠️  需要实际执行解密测试来验证GCM认证标签的有效性")