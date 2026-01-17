# 高级功能示例

本目录包含 confers 高级功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| parallel_validation.rs | 并行验证 | parallel |
| system_monitoring.rs | 系统监控 | monitoring |
| memory_limit.rs | 内存限制 | - |
| custom_sanitizer.rs | 自定义清理器 | - |
| ssrf_protection.rs | SSRF 保护 | - |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`parallel`, `monitoring`

## 运行示例

```bash
# 运行并行验证示例
cargo run --example parallel_validation --features parallel

# 运行系统监控示例
cargo run --example system_monitoring --features monitoring

# 运行内存限制示例
cargo run --example memory_limit

# 运行自定义清理器示例
cargo run --example custom_sanitizer

# 运行 SSRF 保护示例
cargo run --example ssrf_protection
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `large_config.toml` - 大型配置
- `remote_url.toml` - 远程 URL 配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)