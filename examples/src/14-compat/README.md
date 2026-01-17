# 兼容性示例

本目录包含 confers 兼容性功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| config_rs_compat.rs | config-rs 兼容 API | - |
| migration_guide.rs | 迁移指南 | - |

## 前提条件

- Rust 1.75+

## 运行示例

```bash
# 运行 config-rs 兼容示例
cargo run --example config_rs_compat

# 运行迁移指南示例
cargo run --example migration_guide
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `compat.toml` - 兼容性配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)