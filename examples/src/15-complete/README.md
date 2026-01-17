# 完整示例

本目录包含 confers 完整应用的示例，展示如何组合多个特性。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| production_app.rs | 生产环境应用配置 | validation, watch, encryption, remote |
| development_app.rs | 开发环境应用配置 | validation, schema |
| multi_source.rs | 多源配置 | validation, remote |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`validation`, `watch`, `encryption`, `remote`, `schema`

## 运行示例

```bash
# 运行生产环境应用示例
cargo run --example production_app --features "validation,watch,encryption,remote"

# 运行开发环境应用示例
cargo run --example development_app --features "validation,schema"

# 运行多源配置示例
cargo run --example multi_source --features "validation,remote"
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `base.toml` - 基础配置
- `dev.toml` - 开发环境配置
- `prod.toml` - 生产环境配置
- `secrets/encrypted.toml` - 加密密钥配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)