# Design — fix-audit-defects-r1

## Context

confers 是客户端配置库（多来源优先级链合并 + 热重载 + 远程源 + 加密 + 脱敏审计）。审计确认：密码学原语层质量高（XChaCha20-Poly1305 随机 nonce、HKDF NUL 分隔 info、AEAD 严格失败），风险集中在产品层断言与实现脱节——文档承诺的特性是死属性或假实现、防护机制彼此不接线、若干静默失败路径（丢事件/丢值/旧配置续命）无任何信号。仓库纪律：MSRV 1.97.1、clippy `-D warnings`、pre-commit/typos 门禁、多特性矩阵验证（dbnexus 先例：宽特性集 + 全集成目标全绿）。所有修复走前向修复（不回退、不改历史）。

## Decision

### D1 优先级链排序（T001/T010）
`SourceChain::collect_and_merge_report` 排序键从 `(priority, source_id)` 改为 `priority` 上的**稳定排序**（Rust `sort_by_key` 稳定），同优先级保持**声明顺序**；`DefaultSource` priority 固定为最低（-100）而非 0。`MergeEngine::merge` 入口先按 priority 交换 low/high，`report_conflict` 的 winner 与实际结构胜者一致。文档（ARCHITECTURE/API_REFERENCE）按实现修正。备选「保持字母序+文档说明」被否：字母序依赖文件名是不可预测行为，且与全部现有文档矛盾。

### D2 EnvSource 冲突语义（T002/T006/T007）
`insert_nested` 路径前缀冲突（标量 vs 嵌套）定为「**嵌套让标量提升为 map、标量值存入保留键**」不再可行——选定更简单且确定性的语义：**后到的 env 变量按类型冲突报 `ConfigError`（带完整变量名与路径）**，失败可观测；对静默历史行为是显式破坏（CHANGELOG 记录）。排序上先对收集到的 env 变量**排序后插入**，消除 `std::env::vars()` 迭代顺序依赖。env 值类型推断保留，但反序列化错误必须携带字段路径（修复 `builder.rs` 空 key）。`env_separator` 在 `ConfigBuilder`/`SourceChainBuilder` 暴露，文档示例 `APP_DB__HOST` 以 `__` 分隔跑通。

> **实施精化（apply 中发现）**：无前缀源折叠的是整个进程环境，`CARGO` vs `CARGO_HOME` 之类环境固有冲突会让所有开发构建直接报错——过激。`insert_nested` 增加 `strict` 参数：**前缀源与 Memory/Default 的代码键 = 冲突报错**（T002 原语义）；**无前缀源 = 确定性「嵌套 map 形状胜出、标量丢弃」+ `confers.env.path_conflict_dropped` telemetry 事件**（可观测，不再静默，顺序无关由预排序保证）。

### D3 宏生成行为（T003/T004/T009/T011/T012/T018）
- `load_file_with_env` 生成的链只包含**显式声明的 env 映射**（带前缀），不再追加无条件 `.env()`；未声明字段不受进程环境污染。
- `load_file`/`load_file_with_env` 为**全部**含 `#[config(default)]` 字段生成默认值注册（对齐 `load_sync` 现有行为）。
- `profile`/`profile_env` 落地：启用时在基础文件后追加 `<stem>.<env>.<ext>` 叠加（env 取 `profile_env` 指定变量，缺省 `RUN_ENV`；文件不存在则跳过）。
- `#[config(default)]`（空值）生成 `<T::default>::into()`；`default = None` 对 Option 字段生成 `None`；不可推断时 darling 报友好错误。
- 宏侧敏感 `_FILE` 无效路径返回错误（对齐 EnvSource），不静默跳过。
- `#[config(dynamic)]` 生成的 `*_handle()` 经 `OnceLock<Arc<DynamicField<…>>>` 单例化，多次调用共享同一实例。

### D4 热重载可靠性（T015-T023）
- `FsWatcher` 单文件模式改为**监听父目录 + 精确路径过滤**（复用 `MultiFsWatcher` 已验证的 rename 防护），删除 inode 失聪问题；新增「连续两次原子替换」回归测试钉住行为。
- 事件通道 `try_send` Full 分支：`tracing::warn!` + 原子丢弃计数器（公开 `dropped_events()`）。
- `DynamicField::update` 回调执行包 `catch_unwind(AssertUnwindSafe)`：panic 回调被吞并计数，后续回调与 store 不受影响；`AssertUnwindSafe` 理由：回调是用户代码，库不承诺 panic-safety 传递。
- 并发 update 顺序：store 前取 `version.fetch_add`，回调携带版本号，执行时跳过低于最新 store 版本的过期回调（观测顺序=最终值）。
- `InMemoryChangeStream` 驱逐推进 `min_retained_version`，`get(version)` 命中已驱逐区间时返回 `Err(ChangeStreamError::Lagged{from})`，流层把 Lagged 转成显式 resync 事件而非 filter_map 吞掉。
- `AdaptiveDebouncer` 接入 FsWatcher 事件出口（CAS 修复已有回归测试）；WAT-18 去 `#[ignore]`。
- `ConfigImpl` overrides 缓存改 `moka` 永久条目（移除 TTL/TTI 静默回退）。
- `ProgressiveReloader`：新增 `peek_candidate()`（Option<Arc<T>>）使 trial 期可观测；`rollback_on_validation_failure` 在提交前校验失败路径真实消费；`MigrationOnReload` 在提交成功后调用 `MigrationRegistry::migrate`（宏生成的 registry 保持可注入）。

### D5 远程源与总线（T024-T034）
- SSRF：`BLOCKED_NETWORKS` 补 `0.0.0.0/8`、`::/128`、`64:ff9b::/96`；以 `src/security/rules/ssrf.rs` 的 22 条黑名单为单一事实源抽公共函数，poll.rs 复用；修正固化 `::0 not blocked` 的既有测试。
- Consul：recurse 查询返回空数组时**推进 `last_index` 并产出空配置集**（删除语义），仅当响应携带的 `X-Consul-Index` 与上次相同才视为无变化；mock 单测固化。
- K8s：in-cluster 路径读 `/var/run/secrets/kubernetes.io/serviceaccount/ca.crt` 经 `add_root_certificate`；`reqwest::Client` 统一默认超时（connect 10s / total 30s）。
- HTTP 轮询源：builder 未显式设置时应用默认超时；新增 `with_header(name, value)`（认证头注入，值不进 Debug/日志）。
- 容灾：`HttpPolledSourceBuilder::stale_on_error(bool)`（默认 false 保持 fail-loud 现状+既有测试不破坏）；build 失败路径也尝试写快照；CLI 新增 `snapshot restore` 子命令（加载前校验）。
- Redis 总线：订阅循环外包重连（指数退避 1s→30s 封顶）+ 断连 `tracing::warn`；README/bus 文档标注「Redis=at-most-once，NATS/JetStream=至少一次」。
- `VersionArbitratedBus`：发布方进程启动时生成随机 `publisher_epoch: u64`，事件携带 `(epoch, seq)`；仲裁器按 `(publisher_id, epoch)` 分轨单调去重，未知 epoch 视为新轨接受。
- Nacos：`username`/`password` 配置后向 `/nacos/v1/auth/login` 取 `accessToken` 并附加到后续请求（401 时重登一次）；熔断器 `try_lock` 争用（WouldBlock）不记为失败、不误报 open，记 `unknown` 跳过本轮。
- 熔断器以组合方式接入 etcd/Consul/K8s REST 三源（复用 `remote/circuit_breaker.rs`）。
- NATS：consumer 设 `max_deliver(5)`，反序列化失败 `Nak(Some(5s 延迟))`。
- 快照写盘统一 `tmp + rename` 原子替换。

### D6 加密与密钥（T035-T044）
- **encrypt 真实化**：宏对 `#[config(encrypt)]` 字段在加载链合并后、反序列化前执行解密步骤：值匹配统一 envelope 才解密，明文值原样通过（向后兼容）；密钥经 `ConfigLoader` 注入的 `KeyProvider`（默认 `EnvKeyProvider` 读 `CONFERS_MASTER_KEY`，与 doctor 一致——见 NEEDS CLARIFICATION 默认值）。`encrypt = "aes256-gcm"` 生成 `compile_error!`。e2e 从「断言密文可用」改为「断言解密后明文」。
- 无 encryption 特性时加载路径遇 envelope 值：`tracing::warn` + 校验器 warning 项；doctor 无特性分支返回 warning 检查项（状态码不变）。
- 弱密钥：`build()`/`get_key()` 增加 entropy 预检（全零/全 0xFF/单字节重复 → `CryptoError::WeakKey`）；doctor 的 ASCII 口令通道要求至少 16 字节且含两类字符；SECURITY.md 示例改随机生成；`FileKeyProvider` 检查文件权限（unix mode 0o600 之外告警）。
- envelope 统一：`security/prefix.rs` 提供唯一 parse/serialize：`enc:v1:<keyver>:<b64(nonce||ct)>`；无 keyver 旧格式兼容读（默认 v1），写入一律带 keyver；doctor 与 e2e 示例迁移。
- Vault：403 时清 `token_cache` 重登一次；登录响应 `lease_duration` 提前 10% 刷新。`rotate_master_key` 先用旧钥 HMAC 验证调用方持有（挑战-应答）再轮换。
- 受管类型收敛：`derive_field_key`→`Zeroizing<[u8;32]>`、`fetch_and_register`→`SecretBytes`、`KeyBundle::generate`/`get_plaintext_key` 中间串 `Zeroizing<String>`、`decrypt`→`Zeroizing<Vec<u8>>`；`SecureString::masked()` 改定长 8 星号（不泄长度/前缀）；`ZeroizingBytes::drop` 清 full capacity。
- 文件权限：keys.json/export/backup/快照/审计落盘 unix 0600。
- 清理：移除 `secrecy`、`aes-gcm` 死依赖；`KeyCachePolicy` 接入最小 TTL 缓存实现或删除（选删除，CHANGELOG 记录）；文档 `CONFERS_ENCRYPTION_KEY`→`CONFERS_MASTER_KEY`。

### D7 脱敏与审计（T045-T050）
- builder 自动快照：`save_blocking(merged, T::sensitive_paths())`（宏已生成，接上即可）；快照落盘 0600；e2e 补「sensitive 字段在快照中为 `[REDACTED]`」断言。
- CLI：`inspect`/`get` 默认按字段名规则脱敏，`--reveal` 显式明文（stderr 印警告）；`export`/`diff` 的 `sanitize_error_message` 并入 `security::ErrorSanitizer` 的字段名驱动规则（`password=xxx` 值掩码）——两套脱敏器收敛为一套，`user_message()` 同步接入（顺带修复 InvalidValue URL 直通）。
- 冲突报告：`conflict_report` 的 low/high value 按 sensitive_paths 掩码后再 `Debug`。
- 敏感词单一来源：`security/patterns.rs` 扩充 `authorization/passwd/pwd/dsn/bearer` 并放宽复数形 token 边界（命中点后允许 `s`）；`impl_/audit.rs` 关键词表改为调用 patterns（消除第三套口径）。
- audit：`AuditConfig::with_hmac_key()` 支持外置密钥（缺省沿用 salt，文档标注威胁模型）；`AuditSink` 收 sanitize 后事件；`verify_audit_chain` 用常量时间比较（手写 fold-XOR，避免新增 subtle 依赖）。

### D8 文档对齐（T051）
README/ARCHITECTURE/API_REFERENCE/USER_GUIDE/SECURITY 五处按最终实现修正（优先级描述、env 嵌套、渐进发布措辞=「延迟提交+门禁」、熔断覆盖面、总线可靠性等级、审计威胁模型、密钥 env 名）；CHANGELOG 记录全部行为变更。

## Alternatives Considered

- **「仅修文档不对齐实现」**：对 profile/merge_strategy 等死属性最省事，但审计确认这些是文档明示承诺，删承诺属功能回退；选择实现（最小语义版）。
- **「ConfigValue Debug 全量红字」**：破坏库自身调试与既有大量测试；选择只修面向用户的输出路径（CLI/冲突报告/AnnotatedValue）。
- **「HTTP 源 fail-loud 改默认 stale-if-error」**：与既有测试固化的设计意图冲突且静默旧值风险高；选择 opt-in `stale_on_error`。
- **「引入 subtle crate 做常量时间比较」**：为两处比较新增依赖不值；手写 fold-XOR 等价且零依赖。
- **「一次性大 PR 按 FMEA 顺序跨模块修」**：apply 需严格顺序，跨模块交错会让验证面失控；选择按模块分组的连续任务序列，模块内先 P0。

## Consequences

- 正面：52 项审计缺陷全部闭环且每项带回归测试；「静默失败」类问题（丢事件/丢值/失聪/旧配置续命）全部转为可观测（错误/警告/计数器）。
- 行为变更（CHANGELOG 必须列明）：同优先级排序语义、DefaultSource 优先级、env 冲突报错、`load_file_with_env` 不再注入未声明变量、encrypt 属性真实解密、`x_handle` 单例化、`derive_field_key` 等返回类型改受管类型、`KeyCachePolicy` 删除、CLI inspect/get 默认脱敏。
- 技术债：AES-256-GCM 仍缺位（编译期显式拒绝）；Redis 总线仍 at-most-once；ConfigValue Debug 仍明文（文档标注）。
- 后续跟进：若需要服务端级配置中心语义（推送/灰度），属新变更。
