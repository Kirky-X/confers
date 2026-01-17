# 基础功能示例

本目录包含 confers 基础功能的示例，适合初学者入门。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| minimal.rs | 最小化配置，展示最简单的配置加载 | derive |
| basic.rs | 基本配置加载，包含环境变量覆盖 | derive, validation |
| default_values.rs | 默认值设置和使用 | derive |
| env_override.rs | 环境变量覆盖配置 | derive |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`derive`, `validation`

## 运行示例

```bash
# 运行最小化配置示例
cargo run --example minimal

# 运行基本配置示例
cargo run --example basic --features validation

# 运行默认值示例
cargo run --example default_values

# 运行环境变量覆盖示例
cargo run --example env_override
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `minimal.toml` - 最小化配置
- `basic.toml` - 基本配置
- `default_values.toml` - 默认值配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)