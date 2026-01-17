# CLI 工具示例

本目录包含 confers 命令行工具的使用示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| diff_command.rs | diff 命令使用 | cli |
| generate_command.rs | generate 命令使用 | cli |
| validate_command.rs | validate 命令使用 | cli |
| encrypt_command.rs | encrypt 命令使用 | cli, encryption |
| wizard_command.rs | wizard 命令使用 | cli |
| key_command.rs | key 命令使用 | cli, encryption |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`cli`
- 需要安装 confers CLI: `cargo install confers`

## 运行示例

```bash
# 运行 diff 命令示例
cargo run --example diff_command --features cli

# 运行 generate 命令示例
cargo run --example generate_command --features cli

# 运行 validate 命令示例
cargo run --example validate_command --features cli

# 运行 encrypt 命令示例
cargo run --example encrypt_command --features "cli,encryption"

# 运行 wizard 命令示例
cargo run --example wizard_command --features cli

# 运行 key 命令示例
cargo run --example key_command --features "cli,encryption"
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `config1.toml` - 配置文件 1
- `config2.toml` - 配置文件 2

## CLI 命令参考

```bash
# 比较配置文件
confers diff config1.toml config2.toml

# 生成配置模板
confers generate --struct "AppConfig" --output config.toml

# 验证配置文件
confers validate config.toml

# 加密配置文件
confers encrypt config.toml --key-file secret.key --output encrypted.toml

# 交互式向导
confers wizard

# 生成密钥
confers key -o encryption.key
```

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)