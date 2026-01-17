# 文件监控和热重载示例

本目录包含 confers 文件监控和热重载功能的示例。

## 示例列表

| 示例 | 描述 | 特性 |
|------|------|------|
| basic_watch.rs | 基本文件监控 | watch |
| hot_reload.rs | 热重载功能 | watch |
| debounce.rs | 防抖处理 | watch |

## 前提条件

- Rust 1.75+
- 需要启用的特性：`watch`

## 运行示例

```bash
# 运行基本监控示例
cargo run --example basic_watch --features watch

# 运行热重载示例
cargo run --example hot_reload --features watch

# 运行防抖处理示例
cargo run --example debounce --features watch
```

## 配置文件

配置文件位于 `configs/` 子目录中：
- `watch.toml` - 监控配置

## 相关文档

- [用户指南](../../docs/USER_GUIDE.md)
- [API 参考](../../docs/API_REFERENCE.md)