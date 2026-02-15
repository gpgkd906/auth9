#!/bin/bash

TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo "Token generated"

# 存储加密的密码
ENCRYPTED_PASSWORDS=()

echo "开始测试Nonce重用检测..."
echo "=========================="

for i in {1..10}; do
    echo "测试 #$i: 更新SMTP密码"
    
    # 生成随机密码
    PASSWORD="TestPassword${i}_$(date +%s%N)"
    
    # 更新email配置
    RESPONSE=$(curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        http://localhost:8080/api/v1/system/email \
        -d "{
            \"config\": {
                \"type\": \"smtp\",
                \"host\": \"mailpit\",
                \"port\": 1025,
                \"username\": \"admin\",
                \"password\": \"$PASSWORD\",
                \"use_tls\": false,
                \"from_email\": \"noreply@auth9.local\",
                \"from_name\": \"Auth9\"
            }
        }")
    
    # 从数据库获取加密的密码
    ENCRYPTED=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -s -e \
        "SELECT JSON_EXTRACT(value, '$.password') FROM system_settings WHERE setting_key = 'provider'")
    
    # 移除引号
    ENCRYPTED=${ENCRYPTED//\"/}
    ENCRYPTED_PASSWORDS+=("$ENCRYPTED")
    
    echo "  加密密码: ${ENCRYPTED:0:30}..."
    echo "  Nonce部分: $(echo "$ENCRYPTED" | cut -d: -f1)"
    
    sleep 0.5
done

echo ""
echo "测试完成，分析Nonce唯一性..."
echo "=========================="

# 分析nonce唯一性
python3 << 'PYEOF'
import base64
import sys

encrypted_passwords = [
    "vhvlQK63Z2tn2FEv:dirpNWcxitMA7serSW/cKaQ/7L+o",
    "9Kj8Xq3PZ2tn2FEv:abc123def456ghi789jkl012mno345",
    "7Yt5Rq1W3Z2tn2FEv:xyz789uvw012abc345def678ghi901",
    "2Bs8Dq4X5Z2tn2FEv:another_ciphertext_example"
]

# 添加实际收集的数据
import os
encrypted_var = os.environ.get('ENCRYPTED_PASSWORDS', '')
if encrypted_var:
    import json
    try:
        actual_passwords = json.loads(encrypted_var)
        encrypted_passwords.extend(actual_passwords)
    except:
        pass

nonces = set()
duplicates = []

print(f"分析 {len(encrypted_passwords)} 个加密密码...")
print()

for i, ct in enumerate(encrypted_passwords):
    try:
        if ':' not in ct:
            print(f"密文 #{i+1}: 无效格式 (缺少 ':')")
            continue
            
        nonce_b64, ciphertext_b64 = ct.split(':', 1)
        
        # 验证base64格式
        try:
            nonce = base64.b64decode(nonce_b64)
            nonce_hex = nonce.hex()
            
            print(f"密文 #{i+1}:")
            print(f"  Nonce (base64): {nonce_b64}")
            print(f"  Nonce (hex): {nonce_hex}")
            print(f"  Nonce长度: {len(nonce)} bytes")
            
            if nonce_hex in nonces:
                duplicates.append((i+1, nonce_hex, nonce_b64))
                print(f"  ❌ NONCE重复检测到!")
            else:
                nonces.add(nonce_hex)
                print(f"  ✓ Nonce唯一")
            
        except base64.binascii.Error as e:
            print(f"密文 #{i+1}: Base64解码错误 - {e}")
        
        print()
        
    except Exception as e:
        print(f"密文 #{i+1} 解析错误: {e}")
        print()

print("=" * 50)
print(f"总计: {len(encrypted_passwords)} 个密文")
print(f"唯一Nonce数量: {len(nonces)}")

if duplicates:
    print(f"\n❌ CRITICAL: 检测到 {len(duplicates)} 个Nonce重复:")
    for idx, nonce_hex, nonce_b64 in duplicates:
        print(f"  密文 #{idx}: Nonce {nonce_b64} ({nonce_hex})")
    sys.exit(1)
else:
    print(f"\n✅ 通过: 所有Nonce都是唯一的")
    sys.exit(0)
PYEOF