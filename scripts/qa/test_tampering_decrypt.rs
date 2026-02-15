use auth9_core::crypto::aes::{decrypt, encrypt, EncryptionKey};

fn main() {
    println!("密文篡改解密测试");
    println!("=================");

    // 测试密钥
    let key = EncryptionKey::new([0x42u8; 32]);
    let plaintext = "MySecretPassword123!";

    // 正常加密
    let encrypted = match encrypt(&key, plaintext) {
        Ok(enc) => enc,
        Err(e) => {
            println!("❌ 加密失败: {}", e);
            return;
        }
    };

    println!("原始明文: {}", plaintext);
    println!("加密结果: {}", encrypted);
    println!();

    // 解析密文
    let parts: Vec<&str> = encrypted.split(':').collect();
    if parts.len() != 2 {
        println!("❌ 无效的加密格式");
        return;
    }

    let nonce_b64 = parts[0];
    let ciphertext_b64 = parts[1];

    // 测试1: 正常解密
    println!("测试1: 正常解密");
    match decrypt(&key, &encrypted) {
        Ok(decrypted) => {
            if decrypted == plaintext {
                println!("✅ 解密成功: {}", decrypted);
            } else {
                println!("❌ 解密结果不匹配: {}", decrypted);
            }
        }
        Err(e) => println!("❌ 解密失败: {}", e),
    }
    println!();

    // 测试2: bit-flip攻击
    println!("测试2: Bit-flip攻击");
    let ciphertext_bytes = base64::decode(ciphertext_b64).unwrap();
    let mut tampered_ciphertext = ciphertext_bytes.clone();
    if tampered_ciphertext.len() > 0 {
        tampered_ciphertext[0] ^= 0x01; // 修改第一个bit
    }
    let tampered_b64 = base64::encode(&tampered_ciphertext);
    let tampered_encrypted = format!("{}:{}", nonce_b64, tampered_b64);

    match decrypt(&key, &tampered_encrypted) {
        Ok(decrypted) => println!("❌ 危险: 篡改后的密文竟然解密成功: {}", decrypted),
        Err(e) => println!("✅ 安全: 篡改后的密文解密失败: {}", e),
    }
    println!();

    // 测试3: 截断密文（移除GCM tag）
    println!("测试3: 截断密文（移除GCM tag）");
    if ciphertext_bytes.len() > 16 {
        let truncated = &ciphertext_bytes[..ciphertext_bytes.len() - 16];
        let truncated_b64 = base64::encode(truncated);
        let truncated_encrypted = format!("{}:{}", nonce_b64, truncated_b64);

        match decrypt(&key, &truncated_encrypted) {
            Ok(decrypted) => println!("❌ 危险: 截断的密文竟然解密成功: {}", decrypted),
            Err(e) => println!("✅ 安全: 截断的密文解密失败: {}", e),
        }
    } else {
        println!("⚠️  密文太短，无法测试截断");
    }
    println!();

    // 测试4: 修改nonce
    println!("测试4: 修改Nonce");
    let nonce_bytes = base64::decode(nonce_b64).unwrap();
    let mut tampered_nonce = nonce_bytes.clone();
    if tampered_nonce.len() > 0 {
        tampered_nonce[0] ^= 0x01;
    }
    let tampered_nonce_b64 = base64::encode(&tampered_nonce);
    let tampered_nonce_encrypted = format!("{}:{}", tampered_nonce_b64, ciphertext_b64);

    match decrypt(&key, &tampered_nonce_encrypted) {
        Ok(decrypted) => println!("❌ 危险: 修改nonce后的密文竟然解密成功: {}", decrypted),
        Err(e) => println!("✅ 安全: 修改nonce后的密文解密失败: {}", e),
    }
    println!();

    // 测试5: 完全伪造密文
    println!("测试5: 完全伪造密文");
    let fake_ciphertext = vec![0x41u8; ciphertext_bytes.len()]; // 全'A'
    let fake_b64 = base64::encode(&fake_ciphertext);
    let fake_encrypted = format!("{}:{}", nonce_b64, fake_b64);

    match decrypt(&key, &fake_encrypted) {
        Ok(decrypted) => println!("❌ 危险: 伪造的密文竟然解密成功: {}", decrypted),
        Err(e) => println!("✅ 安全: 伪造的密文解密失败: {}", e),
    }
    println!();

    println!("=================");
    println!("测试完成!");
    println!("GCM认证加密应确保任何密文修改都导致解密失败");
}
