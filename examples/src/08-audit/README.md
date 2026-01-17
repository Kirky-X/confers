# 审计日志示例

本目录包含 confers 审计日志功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| basic_audit.rs | 基本审计功能 | audit |
| sensitive_fields.rs | 敏感字段脱敏 | audit |
| audit_history.rs | 审计历史 | audit |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`audit`

## 运行示例

```bash
# 运行基本审计示例
cargo run --example basic_audit --features audit

# 运行敏感字段脱敏示例
cargo run --example sensitive_fields --features audit

# 运行审计历史示例
cargo run --example audit_history --features audit
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `audit.toml` - 审计配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)