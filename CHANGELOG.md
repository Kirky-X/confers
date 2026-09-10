# Changelog

All notable changes to this project will be documented in this file.

## [0.6.0-rc.3] — 2026-09-10

### 修复

- **宏 env 键名覆盖**：修复 `#[confers(name_env = "X")]` 生成的 env 键与声明不一致的缺陷，确保自定义 env 键名与默认命名规则互不污染。
- **skip+default 组合**：修复 `skip` 字段参与加载且 `default` 被 env/文件覆盖的缺陷，修正字段过滤顺序。

### 新增

- **审计 HMAC 链式签名**（`audit` feature）：`AuditEvent` 落盘前计算 `HMAC-SHA256(prev_hash || canonical_event_bytes)`，链首用随机 salt；审计文件写入链头/链尾元数据；提供 `verify_audit_chain(path)` 校验函数。
- **MetricsBackend 关键路径埋点**：loader 加载完成/失败、watcher 触发次数、remote 源拉取延迟与错误、secret 解密错误四类路径接入 `MetricsBackend`，指标名前缀 `confers_`。
- **CLI `schema` 子命令**：输出配置类型的 JSON Schema（需 `cli` feature，自动启用 `schema`）。
- **CLI `get <key>` 子命令**：按点分路径获取配置值，输出单行稳定 JSON；缺失键返回 `null`（退出码 0）。
- **CLI `--fields` 全局选项**：裁剪 JSON 输出到指定字段（逗号分隔点分路径）。
- **CLI 退出码契约**：0 成功 / 1 配置错误 / 2 I/O 错误。

## [0.6.0-rc.2] — 2025-09

- 初始 rc.2 发布（见 crates.io）。
