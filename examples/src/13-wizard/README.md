# 交互式向导示例

本目录包含 confers 交互式向导功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| basic_wizard.rs | 基本向导功能 | - |
| template_generation.rs | 模板生成 | - |

## 前提条件

- Rust 1.75+

## 运行示例

```bash
# 运行基本向导示例
cargo run --example basic_wizard

# 运行模板生成示例
cargo run --example template_generation
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `wizard.toml` - 向导配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)