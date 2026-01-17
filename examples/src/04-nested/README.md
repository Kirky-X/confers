# 嵌套配置示例

本目录包含 confers 嵌套配置结构的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| nested_structs.rs | 嵌套结构体配置 | derive |
| deep_nesting.rs | 深层嵌套配置 | derive |
| array_fields.rs | 数组字段配置 | derive |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`derive`

## 运行示例

```bash
# 运行嵌套结构体示例
cargo run --example nested_structs

# 运行深层嵌套示例
cargo run --example deep_nesting

# 运行数组字段示例
cargo run --example array_fields
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `nested.toml` - 嵌套配置
- `deep_nested.json` - 深层嵌套配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)