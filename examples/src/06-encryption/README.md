# 配置加密示例

本目录包含 confers 配置加密功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| basic_encryption.rs | 基本加密功能 | encryption |
| encrypt_sensitive.rs | 加密敏感字段 | encryption |
| key_management.rs | 密钥管理 | encryption |
| decrypt_config.rs | 解密配置 | encryption |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`encryption`

## 运行示例

```bash
# 运行基本加密示例
cargo run --example basic_encryption --features encryption

# 运行加密敏感字段示例
cargo run --example encrypt_sensitive --features encryption

# 运行密钥管理示例
cargo run --example key_management --features encryption

# 运行解密配置示例
cargo run --example decrypt_config --features encryption
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `plain.toml` - 明文配置
- `encrypted.toml` - 加密配置
- `keys/master.key` - 主密钥文件

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)