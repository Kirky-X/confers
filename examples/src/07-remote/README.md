# 远程配置示例

本目录包含 confers 远程配置功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| http_remote.rs | HTTP 远程配置 | remote |
| etcd_remote.rs | etcd 远程配置 | remote |
| consul_remote.rs | Consul 远程配置 | remote |
| fallback.rs | 回退机制 | remote |
| tls_config.rs | TLS 配置 | remote |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`remote`
- 可选：etcd, Consul 服务

## 运行示例

```bash
# 运行 HTTP 远程配置示例
cargo run --example http_remote --features remote

# 运行 etcd 远程配置示例
cargo run --example etcd_remote --features remote

# 运行 Consul 远程配置示例
cargo run --example consul_remote --features remote

# 运行回退机制示例
cargo run --example fallback --features remote

# 运行 TLS 配置示例
cargo run --example tls_config --features remote
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `remote.toml` - 远程配置
- `fallback.toml` - 回退配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)