# Changelog

All notable changes to this project will be documented in this file.

## [0.6.0-rc.4] — 2026-09-10

> workspace-rc4-completion（Phase 1 confers）：统一变更流、宏属性语义补全、新配置源、doctor 自检、观测与安全增强。

### 新增

- **统一变更流端口**（`change-stream` feature，T101）：`ChangeStream` trait（publish/subscribe/ack），`InMemoryChangeStream` 复用 `ConfigBus`；watch 与 remote 变更统一发布 `ChangeEvent` 信封（key/旧值/新值/来源 + 单调版本）。
- **宏四属性 codegen 真实消费**（T102）：`flatten`（顶层键提升进嵌套结构，`ConfigFieldKeys` 钩子）、`dynamic`（`<field>_handle()` DynamicField 运行时句柄）、`interpolate`（字段值 `${key}`/`${key:default}` 按合并树解析）、`watch`（字段级热重载订阅器 `field_watcher`）；四属性组合不冲突，macro_e2e 语义断言。
- **etcd 原生 watch 流**（`etcd-watch` feature，T103）：`EtcdWatcher` 消费 watch 流替代前缀 GET 轮询，断线有界指数退避自动重连，`WatchEventSource` 抽象可 mock；`EtcdSource::watch_transport()` 一键接入。
- **Kubernetes 配置源**（`k8s` feature，T104）：`K8sMountedSource` 挂载卷源（kubelet 原子写 symlink swap 感知，`..data` 代际检测）+ `K8sApiSource` REST API 源骨架（ConfigMap/Secret、in-cluster 探测、Secret base64 解码）。
- **Nacos 配置源**（`nacos` feature，T105）：HTTP Open API 拉取 + 定时监听（未变更内容命中缓存快照），命名空间/分组映射，熔断器保护。
- **CLI `doctor` 子命令**（T106）：schema 结构 / 来源优先级链 / 加密字段可解密（`enc:v1:` 信封 + `CONFERS_MASTER_KEY`）/ env 覆盖冲突 / 加载五项检查；单行 JSON 报告 + 退出码 0 健康 / 1 警告 / 2 错误。
- **CLI `schema --from-instance`**（T107）：从已加载配置实例反推 JSON Schema 草稿（对象 required、数组 items、标量类型推断）。
- **tracing 集成**（`tracing` feature，T108）：load/reload/decrypt/remote fetch 四条关键路径 span 与事件；与 MetricsBackend 并行不互斥；feature 关闭零开销。
- **AuditSink 多 sink 端口**（T109）：对象安全 `pub trait AuditSink: Send + Sync`，`AuditWriter` 支持注入多个 sink（`add_sink`/`with_sink`/builder），默认本地 HMAC 链文件保留不变；供 inklog 上层实现。
- **性能基线门禁**（T110）：`watch_callback_bench`（变更流往返/扇出/记账）+ load/merge/hot_path 基线数字记录 `docs/PERFORMANCE.md`。
- **Agent 知识包**（T111）：CLI `docs --agent` 输出子命令/参数/退出码契约/常见任务配方（JSON + Markdown，≤200 行）。
- **宏批量重命名**（T112）：`#[config(rename_all = "camelCase|snake_case|kebab-case")]`，codegen 在反序列化前把外部键映射回 serde 字段名（serde 名显式出现时优先），非法风格宏展开期报错（trybuild）。
- **云密钥后端**（`cloud-kms` feature，T113）：Vault transit KeyProvider MVP（`POST /v1/transit/decrypt/{key}` 解包 32 字节主密钥），`CloudKmsBackend` 端口留 AWS/GCP 扩展点；mock server 测试。
- **Vault 认证增强**（T114）：`VaultAuth::AppRole` / `Kubernetes`（pod JWT + auth role）登录换 client token 并缓存；`kubernetes_from_service_account` 从 in-pod token 文件构建；mock 测试。
- **系统钥匙串**（`keyring` feature，T115）：`KeyringStore` 端口 + `SecretToolKeyringStore`（freedesktop Secret Service）/ `FileKeyringStore`（chmod-600 回退并告警）；`MasterKeyStore::from_environment` 自动选择；无 DBus 环境 skip 测试。
- **OpenFeature 灰度引擎**（`openfeature` feature，T116）：`FeatureProvider` 端口 + `NoOpProvider` + `StaticFlagProvider`（属性规则 + 按 targeting key 确定性百分比分桶）+ `ToggleRegistryProvider` 桥接既有 FeatureToggleRegistry。
- **总线版本仲裁**（T117）：`VersionArbitratedBus` 包装任意 `ConfigBus`，发布端按实例打单调版本，订阅端丢弃乱序/过期/重复版本并计数；未版本化事件 fail-open。
- **零拷贝热路径**（T118）：`InMemoryConfig` 内部存储改 `Arc<AnnotatedValue>`，新增 `SharedValueReader::get_shared()` 共享句柄读取；10 KiB value 读取 ~2.1x 提升（bench 对比记录 PERFORMANCE.md）。
- **金丝雀联动**（T119，`change-stream` feature）：ProgressiveReloader 阶段迁移（trial_started/committed/rolled_back）发布 `ChangeSource::Canary` 事件到 ChangeStream，供上层编排。
- **惰性分段解析**（`lazy` feature，T120）：`LazySegmentedConfig` 按顶层 TOML 表头切分（纯行扫描），段首次访问才解析并缓存；单测断言未访问段零解析。

### 变更

- `InMemoryConfig` 值存储改为 `Arc<AnnotatedValue>`（公共 API 兼容；`get_raw`/`get_string` 语义不变）。


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
