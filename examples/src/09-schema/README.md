# Schema 生成示例

本目录包含 confers Schema 生成功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| json_schema.rs | JSON Schema 生成 | schema |
| typescript_types.rs | TypeScript 类型生成 | schema |
| schema_validation.rs | Schema 验证 | schema |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`schema`

## 运行示例

```bash
# 运行 JSON Schema 生成示例
cargo run --example json_schema --features schema

# 运行 TypeScript 类型生成示例
cargo run --example typescript_types --features schema

# 运行 Schema 验证示例
cargo run --example schema_validation --features schema
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `schema.toml` - Schema 配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)