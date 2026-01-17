# 多格式支持示例

本目录包含 confers 支持的多种配置格式示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| toml_example.rs | TOML 格式配置 | derive |
| json_example.rs | JSON 格式配置 | derive |
| yaml_example.rs | YAML 格式配置 | derive |
| ini_example.rs | INI 格式配置 | derive |
| auto_detection.rs | 自动格式检测 | derive |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`derive`

## 运行示例

```bash
# 运行 TOML 格式示例
cargo run --example toml_example

# 运行 JSON 格式示例
cargo run --example json_example

# 运行 YAML 格式示例
cargo run --example yaml_example

# 运行 INI 格式示例
cargo run --example ini_example

# 运行自动格式检测示例
cargo run --example auto_detection
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `config.toml` - TOML 格式配置
- `config.json` - JSON 格式配置
- `config.yaml` - YAML 格式配置
- `config.ini` - INI 格式配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)