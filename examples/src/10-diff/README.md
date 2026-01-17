# 配置差分示例

本目录包含 confers 配置差分功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| basic_diff.rs | 基本差分功能 | - |
| diff_formats.rs | 差分格式 | - |
| diff_report.rs | 差分报告 | - |

## 前提条件

- Rust 1.75+

## 运行示例

```bash
# 运行基本差分示例
cargo run --example basic_diff

# 运行差分格式示例
cargo run --example diff_formats

# 运行差分报告示例
cargo run --example diff_report
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `config_v1.toml` - 配置版本 1
- `config_v2.toml` - 配置版本 2

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)