use auth9_core::crypto::aes::{encrypt, EncryptionKey};
use std::collections::HashSet;

fn main() {
    // 测试密钥
    let key = EncryptionKey::new([0x42u8; 32]);
    let plaintext = "TestPassword123!";

    println!("测试Nonce唯一性");
    println!("=================");
    println!("明文: {}", plaintext);
    println!("测试次数: 100");
    println!();

    let mut nonces = HashSet::new();
    let mut duplicates = Vec::new();

    for i in 0..100 {
        match encrypt(&key, plaintext) {
            Ok(ciphertext) => {
                let parts: Vec<&str> = ciphertext.split(':').collect();
                if parts.len() == 2 {
                    let nonce_b64 = parts[0];

                    if nonces.contains(nonce_b64) {
                        duplicates.push((i + 1, nonce_b64.to_string()));
                        println!("第 {} 次: ❌ Nonce重复: {}", i + 1, nonce_b64);
                    } else {
                        nonces.insert(nonce_b64.to_string());
                        if i < 5 {
                            println!("第 {} 次: ✓ Nonce唯一: {}", i + 1, nonce_b64);
                        }
                    }
                }
            }
            Err(e) => {
                println!("第 {} 次: 加密失败: {}", i + 1, e);
            }
        }
    }

    println!();
    println!("=================");
    println!("测试结果:");
    println!("总加密次数: 100");
    println!("唯一Nonce数量: {}", nonces.len());

    if duplicates.is_empty() {
        println!("✅ 通过: 所有Nonce都是唯一的");
    } else {
        println!("❌ 失败: 检测到 {} 个Nonce重复", duplicates.len());
        for (count, nonce) in duplicates.iter().take(5) {
            println!("  第 {} 次: Nonce {}", count, nonce);
        }
    }
}
