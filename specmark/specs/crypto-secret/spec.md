# Spec — crypto-secret

> Main spec for capability `crypto-secret`.

## Requirements

### R-crypto-001: encrypt 属性真实解密

`#[config(encrypt)]` 字段在加载管线（合并后、反序列化前）执行解密：统一 envelope 密文经注入的 KeyProvider 解密注入，明文值原样直通；`encrypt = "aes256-gcm"` 产生编译期错误。
**验收标准：**
- e2e 断言字段值为解密后明文而非 `enc-` 前缀字符串
- 明文直通路径测试通过（向后兼容）
- aes256-gcm 标注触发 compile_error（单测编译验证）
- 默认 KeyProvider 读 `CONFERS_MASTER_KEY`（与 doctor 一致），支持显式注入

### R-crypto-002: 无特性告警

encryption 特性未启用时遇 envelope 值：加载路径 tracing warn + 校验 warning 项；doctor 返回 warning 检查项。
**验收标准：**
- 两处测试断言警告产生且不阻断加载

### R-crypto-003: 弱密钥拒绝

全零/全 0xFF/单字节重复 32 字节密钥在 build/get_key 返回 `CryptoError::WeakKey`；doctor ASCII 口令通道要求 ≥16 字节；密钥文件权限非 0600 产生告警。
**验收标准：**
- `[0u8;32]` 拒绝测试；SECURITY.md 示例为随机密钥生成方式
- 0o644 密钥文件触发告警日志

### R-crypto-004: envelope 统一带密钥版本

唯一 parse/serialize 实现 `enc:v1:<keyver>:<payload>`；无 keyver 旧格式兼容读（默认 v1）；写入一律带 keyver；doctor/e2e 迁移同一格式。
**验收标准：**
- 往返测试（含 keyver）+ 旧格式兼容测试通过
- 三处调用点共用同一实现（无第二套解析）

### R-crypto-005: Vault token 续期

403 响应清除 token 缓存并重登一次；按 lease_duration 提前 10% 主动刷新。
**验收标准：**
- mock：token 过期后首次 403 → 重登 → 成功取 key

### R-crypto-006: rotate 验证旧钥

`rotate_master_key` 以旧钥对已知数据做验证后再轮换；验证失败返回错误且不轮换。
**验收标准：**
- 正确旧钥成功；错误旧钥失败且新钥未生效

### R-crypto-007: 明文密钥受管

`derive_field_key` 返回 `Zeroizing<[u8;32]>`；`fetch_and_register` 返回 SecretBytes；KeyBundle 中间串 Zeroizing；`decrypt` 返回 `Zeroizing<Vec<u8>>`。
**验收标准：**
- 类型签名编译断言 + Drop 清零抽查测试
- 全库 clippy/test 在新签名下 green（调用点全适配）

### R-crypto-008: 掩码与清理

`SecureString::masked()` 固定 8 星号；keys.json/export/backup 落盘 0600；`ZeroizingBytes` drop 清 full capacity；移除 secrecy/aes-gcm 死依赖与 KeyCachePolicy；文档 env 名统一 `CONFERS_MASTER_KEY`。
**验收标准：**
- 长短值掩码输出一致；文件权限断言测试
- `cargo tree`/编译无 secrecy、aes-gcm 残留；全库无 `CONFERS_ENCRYPTION_KEY` 引用

## Constraints

- XChaCha20 随机 nonce、HKDF info NUL 分隔、AEAD 严格失败等既有正确实现不得改动。
- 错误信息不泄露密钥内容；Debug/Display 对密钥类型保持 redacted。

## Out of Scope

- AES-256-GCM 实现、HSM 集成、密钥托管服务扩展。
