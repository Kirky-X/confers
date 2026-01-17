# 配置验证示例

本目录包含 confers 配置验证功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| basic_validation.rs | 基本验证功能 | validation |
| custom_validators.rs | 自定义验证器 | validation |
| validation_errors.rs | 验证错误处理 | validation |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`validation`

## 运行示例

```bash
# 运行基本验证示例
cargo run --example basic_validation --features validation

# 运行自定义验证器示例
cargo run --example custom_validators --features validation

# 运行验证错误处理示例
cargo run --example validation_errors --features validation
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `valid.toml` - 有效配置
- `invalid.toml` - 无效配置（用于测试）

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)