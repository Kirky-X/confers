<!-- domain: code -->

# fix-audit-defects-r1

## Motivation

2026-09-23 对 confers 做了「大厂面试题对照审计」（5 个方向并行代码审计 + FMEA 排序），发现 52 项缺陷工作包：20 项 Critical（RPN≥200）、18 项 High、7 项 Medium、7 项 Low。其中包括：`encrypt` 宏属性不产生任何加解密（假加密，RPN=630）、同优先级配置源按文件名字母序导致默认值覆盖配置文件（RPN=567）、单文件 FsWatcher 在编辑器原子替换后热重载静默永久失效（RPN=560）、builder 自动快照对 `#[config(sensitive=true)]` 字段零脱敏明文落盘（RPN=540）、SSRF 黑名单缺 `0.0.0.0/8`（RPN=243）等可直接伤害用户的真实 bug。多数问题属「防护机制存在但集成缝断裂」：脱敏器在但 builder/CLI 不接、熔断器在但只有 HTTP/Nacos 有、回滚配置在但零消费。本变更逐一修复全部 52 项，并对每项补回归测试固化行为。

## Scope

按模块分五组（任务编号详见 tasks.md）：

- **加载与合并**（T001-T014）：优先级链排序语义（声明顺序取代字母序）、EnvSource 嵌套冲突不再静默丢值、宏 `load_file_with_env` 不再注入全进程环境、`#[config(default)]` 在文件加载路径生效、null 覆盖语义统一、env 类型错误携带字段路径、env 嵌套 separator 可配置、merge_strategy/profile 属性接线、插值转义与敏感引用告警、MergeEngine priority 一致性、宏默认值编译错误、宏 `_FILE` 行为对齐。
- **热重载**（T015-T023）：FsWatcher 改父目录监听修复原子替换失聪、事件通道满告警+计数、回调 panic 隔离、dynamic 字段 handle 单例化、并发 update 回调有序、ChangeStream 驱逐显式 Lagged、AdaptiveDebouncer 接线、overrides 移除 TTL 静默回退、渐进发布 candidate 可观测 + rollback/migration 死配置接线。
- **远程与总线**（T024-T034）：SSRF 黑名单补 0.0.0.0/8 等、Consul 空数组=删除语义、K8s in-cluster CA+超时、HTTP 轮询默认超时+认证头、容灾兜底（stale_on_error 选项/失败路径快照/restore 子命令）、Redis 总线重连、总线版本 epoch、Nacos 认证+熔断误报、熔断器接入 etcd/Consul/K8s、NATS 毒消息 max_deliver、快照原子写。
- **加密与密钥**（T035-T044）：encrypt 属性真实化（加载管线解密注入 + aes256-gcm 编译期拒绝）、无 encryption 特性遇 enc 值告警、弱密钥拒绝、envelope 统一带密钥版本、Vault token 过期重登、rotate 验证旧钥、明文密钥收敛受管类型、SecureString 定长掩码、敏感文件 0600、死依赖清理与文档 env 名修正。
- **脱敏与审计 + 文档**（T045-T051）：builder 快照接入 sensitive_paths、CLI inspect/get 默认脱敏 + --reveal、export/diff 接入字段名驱动脱敏规则、冲突报告值脱敏、敏感词表统一扩充、掩码定长化/AuditSink 脱敏/常量时间比较/HMAC 外置密钥、五份文档与 CHANGELOG 对齐。

## Non-Goals

- **不重写配置中心架构**：不做长连接推送、不做服务端组件——confers 是客户端库，现有轮询/监听模型保持。
- **不做 Redis 总线可靠性升级**（如改 Streams）：仅修复断线重连与文档标注可靠性等级；JetStream 级语义属新特性。
- **不改 ConfigValue 的 Debug 全量输出**：库内部调试需要明文 Debug；修复集中在冲突报告、AnnotatedValue 输出与 CLI 面向用户的路径。
- **不实现 AES-256-GCM**：本变更将其改为编译期显式拒绝（假承诺比缺失更危险）；实现属后续变更。
- **不做 semver 兼容层**：仓库处于 0.6.0-rc 阶段，允许 API 签名前向演进（如返回类型改受管类型），以 CHANGELOG 记录。
- **不 commit/推送**：遵循仓库惯例，代码留在工作树待用户确认（对齐 dbnexus fix-audit-defects-r1 先例）。

## NEEDS CLARIFICATION

- **[technical]** T035 encrypt 属性解密注入需要约定密钥默认来源——审计确认代码实际读取 `CONFERS_MASTER_KEY`（doctor 路径），加载管线默认 EnvKeyProvider 是否也用它 — 阻塞任务默认密钥来源写法 — 默认：与 doctor 一致用 `CONFERS_MASTER_KEY`，同时支持显式传入 provider。
