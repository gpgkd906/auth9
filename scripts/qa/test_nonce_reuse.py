#!/usr/bin/env python3
import base64
import json
import sys

# 模拟从数据库收集的加密密码
# 格式: base64(nonce):base64(ciphertext+tag)
ciphertexts = [
    "vhvlQK63Z2tn2FEv:dirpNWcxitMA7serSW/cKaQ/7L+o",
    "9Kj8Xq3PZ2tn2FEv:abc123def456ghi789jkl012mno345",
    "7Yt5Rq1W3Z2tn2FEv:xyz789uvw012abc345def678ghi901",
    "vhvlQK63Z2tn2FEv:different_ciphertext_here",  # 故意重复的nonce用于测试
    "2Bs8Dq4X5Z2tn2FEv:another_ciphertext_example"
]

print("Nonce重用检测测试")
print("=" * 50)

nonces = set()
duplicates = []

for i, ct in enumerate(ciphertexts):
    try:
        nonce_b64, ciphertext_b64 = ct.split(':')
        nonce = base64.b64decode(nonce_b64)
        nonce_hex = nonce.hex()
        
        print(f"密文 #{i+1}:")
        print(f"  Nonce (base64): {nonce_b64}")
        print(f"  Nonce (hex): {nonce_hex}")
        print(f"  Nonce长度: {len(nonce)} bytes")
        
        if nonce_hex in nonces:
            duplicates.append((i+1, nonce_hex))
            print(f"  ⚠️  NONCE重复检测到!")
        else:
            nonces.add(nonce_hex)
            print(f"  ✓ Nonce唯一")
        
        print()
        
    except Exception as e:
        print(f"密文 #{i+1} 解析错误: {e}")
        print()

print("=" * 50)
print(f"总计: {len(ciphertexts)} 个密文")
print(f"唯一Nonce数量: {len(nonces)}")

if duplicates:
    print(f"\n❌ CRITICAL: 检测到 {len(duplicates)} 个Nonce重复:")
    for idx, nonce_hex in duplicates:
        print(f"  密文 #{idx}: Nonce {nonce_hex}")
    sys.exit(1)
else:
    print(f"\n✅ 通过: 所有Nonce都是唯一的")
    sys.exit(0)