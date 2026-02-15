use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;

fn main() {
    println!("GCM认证标签验证测试");
    println!("===================\n");

    // 生成随机密钥
    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill(&mut key_bytes);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();

    let plaintext = b"SuperSecretPassword123!";

    // 生成随机nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 正常加密
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
    println!("原始明文: {}", String::from_utf8_lossy(plaintext));
    println!("Nonce (hex): {}", hex::encode(nonce_bytes));
    println!("密文长度: {} bytes", ciphertext.len());
    println!("密文 (hex): {}...", hex::encode(&ciphertext[..20]));
    println!();

    // 测试1: 正常解密
    println!("测试1: 正常解密");
    match cipher.decrypt(nonce, ciphertext.as_ref()) {
        Ok(decrypted) => {
            println!("✅ 解密成功: {}", String::from_utf8_lossy(&decrypted));
        }
        Err(e) => {
            println!("❌ 解密失败: {:?}", e);
        }
    }
    println!();

    // 测试2: bit-flip攻击
    println!("测试2: Bit-flip攻击");
    let mut tampered_ciphertext = ciphertext.clone();
    if tampered_ciphertext.len() > 0 {
        tampered_ciphertext[0] ^= 0x01; // 修改第一个bit
        println!("修改了密文第一个字节的一个bit");
    }

    match cipher.decrypt(nonce, tampered_ciphertext.as_ref()) {
        Ok(decrypted) => {
            println!(
                "❌ 危险: 篡改后的密文竟然解密成功: {}",
                String::from_utf8_lossy(&decrypted)
            );
        }
        Err(_) => {
            println!("✅ 安全: 篡改后的密文解密失败 (GCM认证标签验证成功)");
        }
    }
    println!();

    // 测试3: 截断密文（移除GCM tag）
    println!("测试3: 截断密文（移除GCM tag）");
    if ciphertext.len() > 16 {
        let truncated = &ciphertext[..ciphertext.len() - 16];
        println!("移除了最后16字节的GCM tag");

        match cipher.decrypt(nonce, truncated) {
            Ok(decrypted) => {
                println!(
                    "❌ 危险: 截断的密文竟然解密成功: {}",
                    String::from_utf8_lossy(&decrypted)
                );
            }
            Err(_) => {
                println!("✅ 安全: 截断的密文解密失败 (缺少GCM认证标签)");
            }
        }
    }
    println!();

    // 测试4: 修改nonce
    println!("测试4: 修改Nonce");
    let mut tampered_nonce_bytes = nonce_bytes.clone();
    tampered_nonce_bytes[0] ^= 0x01;
    let tampered_nonce = Nonce::from_slice(&tampered_nonce_bytes);
    println!("修改了nonce第一个字节的一个bit");

    match cipher.decrypt(tampered_nonce, ciphertext.as_ref()) {
        Ok(decrypted) => {
            println!(
                "❌ 危险: 修改nonce后的密文竟然解密成功: {}",
                String::from_utf8_lossy(&decrypted)
            );
        }
        Err(_) => {
            println!("✅ 安全: 修改nonce后的密文解密失败 (nonce不匹配)");
        }
    }
    println!();

    // 测试5: 完全伪造密文
    println!("测试5: 完全伪造密文");
    let fake_ciphertext = vec![0x41u8; ciphertext.len()]; // 全'A'
    println!("使用全'A'的伪造密文");

    match cipher.decrypt(nonce, fake_ciphertext.as_ref()) {
        Ok(decrypted) => {
            println!(
                "❌ 危险: 伪造的密文竟然解密成功: {}",
                String::from_utf8_lossy(&decrypted)
            );
        }
        Err(_) => {
            println!("✅ 安全: 伪造的密文解密失败 (无效的GCM认证标签)");
        }
    }
    println!();

    println!("===================");
    println!("测试总结:");
    println!("✅ GCM认证加密正确工作:");
    println!("   - 任何密文修改都导致解密失败");
    println!("   - 认证标签验证确保密文完整性");
    println!("   - nonce修改也导致解密失败");
    println!("   - 这是AES-GCM的安全特性");
}
