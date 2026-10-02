# 更新日志

本文件记录本项目的全部重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，
版本号遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

## [Unreleased]

### 依赖升级与特性裁剪

- 传递依赖 patch 升级（`cargo update`，Cargo.lock）：lazy_static 1.5.0→1.5.1、pulldown-cmark-to-cmark 22.0.1→22.0.3、quinn-proto 0.11.18→0.11.19、quinn-udp 0.5.15→0.5.16、tokio-rustls 0.26.5→0.26.6、xxhash-rust 0.8.18→0.8.19、yoke-derive 0.8.3→0.8.4；直接依赖 x.x 基线经 crates.io 核对均已在最新稳定 major.minor，无版本号变更
- 特性收窄（`default-features = false` 不变，显式特性最小化）：`regex` `unicode` 全聚合 → `unicode-case` + `unicode-perl`（src 全部 75 个静态模式仅用 `(?i)`、`\d`/`\s`/`\w`/`\b`/`\S`，双特性集隔离对照验证编译与 ASCII 语义逐字节一致）；`rand` `["std","std_rng","sys_rng","thread_rng"]` → `["thread_rng"]`（前三项为其传递子集，代码仅用 `rng()`/`Rng::fill_bytes`）；examples 的 `notify` 移除 Linux 目标下空操作的 `macos_fsevent`
- CI actions 升级（SHA pin）：taiki-e/install-action v2.87.20→v2.87.22、github/codeql-action v4.38.2 SHA 修正至官方 tag 当前指向、dtolnay/rust-toolchain master 前进（checkout/rust-cache/setup-protoc/codecov/gh-release 经比对已最新未动）

### security 与 encryption 特性解耦（FEATURE_AUDIT_REPORT §9.2-5）

- `security` 不再隐含 `encryption`：security 模块（`SecureString`、错误脱敏 `ErrorSanitizer`）实际仅需 zeroize（零化）与 sha2+hex（`fingerprint` 指纹）三个轻量原语，现由 `security` 直接声明；启用 `security-rules`（规则引擎为纯计算校验）与 `recommended` 预设不再经链式传递拉入 chacha20poly1305/hkdf/tokio/async-trait 等加密与异步栈依赖（getrandom 经 moka→uuid 基线链仍在图中、非本次解耦可移除；recommended 直接依赖 27→22、传递去重 131→119，实测口径同 FAQ 依赖数量表）
- **有效公开面不变**：`ErrorSanitizer`/`SecureString` 等导出的生效门控由 `encryption` 收敛为 `security`——原链式语义（security⇒encryption）下两者恒同开，消费者无感知；`--features encryption` 单开组合行为不变（security 模块本就不编译）。需要加密能力的组合显式启用 `encryption`（`production`/`full` 预设已含）
- 门禁与文档同步：presets_e2e prs08 固化解耦负面断言（`security` 成员不得含 `encryption`）；README/README_EN 功能矩阵 `security` 行改写；TEST_SCENARIOS §3.1 依赖链表与 PRS-08 行更新；API_REFERENCE `ErrorSanitizer` 门控说明、FAQ `recommended` 依赖数量行、CONTRIBUTING 默认特性名同步；README/README_EN 预设表残留旧名（`env`/`schema`）一并修正为正名

### inklog 审计双向集成（纯文档配套）

- **双向关系**：inklog 侧既有 `config-confers` 特性经 confers 加载配置并 watch 热更新（inklog 消费 confers）；本次 confers `audit` 特性的既有 `AuditSink` 对象安全多 sink 端口（`add_sink`/`with_sink`/builder 注入，见 [0.6.0-rc.3] 节）由 inklog `integrations::ConfersAuditSink`（`confers-audit` 特性，自 confers 0.6.0-rc.5 钉版消费）实现——审计事件（KeyAccess/KeyRotation/Decrypt/LoadSuccess/ReloadTrigger）在本地 HMAC 链文件照旧的同时同步转发 inklog 结构化 sink（confers 事件流入 inklog）。端口在 confers、实现在 inklog，confers 本侧零代码变更、零新增依赖（不反向依赖 inklog）
- **集成示例**：可编译实跑示例位于 inklog 仓库 `examples/src/bin/config/confers_audit.rs`（`cargo run --package inklog-examples --features confers-audit --bin confers_audit`）
- **文档同步**：README/README_EN 功能矩阵 `audit` 行与 USER_GUIDE「审计日志配置」节标注该集成

### 近义 feature 正名迁移（env→dotenv、key→key-management、schema→json-schema）

- 三个近义 feature 对正名迁移：`dotenv`（语义即 .env 文件加载，`env` 曾与 `SourceKind::Environment` 环境变量源术语冲突）、`key-management`（密钥轮换/版本管理语义，`key` 过泛）、`json-schema`（产出即 JSON Schema，`schema` 过泛）；预设（default/minimal/recommended/dev/production/full/distributed）与内部 cfg 门控全部记名正名
- 旧名保留一个版本作兼容别名（`env = ["dotenv"]`、`key = ["key-management"]`、`schema = ["json-schema"]`），启用任一别名经 `build.rs` 输出编译期 `cargo:warning` 弃用提示，下一版本移除
- 迁移映射文档化：README/README_EN 功能矩阵标注正名与别名关系；ARCHITECTURE 特性-模块映射表、API_REFERENCE/FAQ/USER_GUIDE/LIBRARY_INTEGRATION/TEST_SCENARIOS 同步；presets_e2e 固化别名链（`env→dotenv`、`key→key-management`、`schema→json-schema`）与正名预设展开
- 存量测试修复：doctor 的 healthy 断言改为特性组合感知——无 `encryption` 的降级构建下 doctor 有意报 warning（exit 1），测试按构建态分别钉住 healthy/warning 契约，修复 `dev` 预设组合下 cli 套件的误报失败

### 云 KMS 后端（cloud-kms 特性：Vault + AWS + GCP）

- AWS KMS（`AwsKmsKeyProvider`）：`kms:Decrypt` 经 SigV4 签名的 JSON 1.1 API 直调（rustls HTTP，无 AWS SDK）——SigV4 纯函数实现（hmac/sha2/hex）并通过 AWS 文档公开测试向量（AKIDEXAMPLE 向量，非真实凭据）钉住签名正确性；凭据来自 builder 或 `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY`/`AWS_SESSION_TOKEN`，支持临时凭据 `x-amz-security-token`；endpoint 可覆盖（plain HTTP 仅允许回环地址——127.0.0.0/8/::1/localhost 精确匹配，名称形如 127.0.0.1.evil.com 的域名拒绝），非回环地址强制 HTTPS
- GCP KMS（`GcpKmsKeyProvider`）：`cryptoKeys.decrypt` + 实例 metadata server token（`Metadata-Flavor: Google` 握手必需——该头正是区分真实 metadata 流量与 SSRF 探测的机制）；key resource 路径组件白名单校验（拒绝遍历/转义/空段），metadata 与 KMS endpoint 均可覆盖，token 亦可直接注入（测试/预取凭据）
- 两者均实现 `CloudKmsBackend` 与 `AsyncKeyProvider`（复用 KeyCachePolicy），`CloudKmsBackend::decrypt` 返回 `ZeroizingBytes`——明文密钥自诞生即处于零化容器（drop 自动擦除，无明文副本驻留）；错误映射保留 429/502/503/504 retryable 语义；mock-server 契约测试验证请求形状（签名服务端重算、Metadata-Flavor 握手、Bearer 来源）与解密字节；真实云集成测试以 `#[ignore]` + `CONFERS_AWS_KMS_LIVE` / `CONFERS_GCP_KMS_LIVE` 环境变量双重门控，沙箱默认跳过
- `cloud-kms` 特性追加 `dep:hmac` / `dep:hex`（SigV4 所需）；密钥零硬编码——测试仅使用 AWS 文档公开示例常量

### 金丝雀发布编排器（canary 特性 = change-stream + progressive-reload）

- 新增 `CanaryOrchestrator`（`src/canary.rs`）：ChangeStream 消费侧的跨实例分批 rollout 编排——按 `RolloutPlan`（batch_size / batch_interval / poll_interval / failure_threshold 全可配）收集各批实例的 `committed` 事件、批次健康检查（复用 `HealthStatus` 三态语义：仅 Critical 计入回滚判定，Degraded 随指令 detail 可观测），推进/回滚决策以 `canary.orchestrator` 指令事件（advance / rollback / completed）发布回流
- **实例身份协议**：实例事件 key 为 `canary.<instance_id>`——`ProgressiveReloader` 新增 `with_instance_id`（默认 `unknown`，未设置的事件无法被编排器归属），编排器按 key 中的实例 id 与当前批实例集合做集合匹配：重复 `committed` 折叠、未知实例与跨批事件永不计数；已推进批次的实例 `rolled_back` 同样立即中止 rollout（候选版本在已推广批次上的回归信号）
- **失败显性化**：回滚路径的 mesh 权重下发或 rollback 指令发布失败 → `RolloutOutcome::Aborted.mesh_rolled_back_failed = true` 且 reason 追加失败说明；推进/完成路径 mesh 切流失败 → 尽力回切 baseline 后 `run()` 返回 `Err`（rollout 不会在未生效的切流上继续）；失败计入 `confers_canary_errors_total{reason=...}` 并在 `tracing` 特性下输出 warn
- 回滚路径三触发：当前批或已推进批实例 `rolled_back` 事件、健康 Critical 比例超 `failure_threshold`（逐轮评估，超阈立即中止，不等窗口耗尽）、committed 收齐超时——均发布 rollback 指令并将流量切回 baseline；空实例列表（服务发现失败）直接报错，绝不向零个已验证实例推送 100% 切流
- 健康检查单次调用受 `poll_interval` 与窗口剩余的较小值约束（挂起的实现转 Degraded，不阻塞 rollout）
- 新增 `MeshWeightPublisher` 服务网格适配层：rollout 进度 → baseline↔canary 流量权重线性下发（`with_mesh`），Envoy weighted clusters 与 Istio VirtualService merge-patch 配置样例及 Rust 适配骨架见 `docs/CANARY_ORCHESTRATION.md`
- 本地多实例模拟：进程内多 `ProgressiveReloader` attach 同一 `InMemoryChangeStream` 的端到端测试（`orchestrates_in_process_reloaders_end_to_end`），19 个契约测试覆盖分批推进/回滚三路径（含已推进批次回归）/Degraded/重复与跨批事件去重/空列表 fail-loud/健康检查挂起/mesh 失败两路径/mesh 权重序列

### JSON Schema → Rust 反向生成

- CLI `schema --from-schema <FILE>`：从 JSON Schema 草稿（2020-12）生成可编译的 Rust struct 脚手架（schema-first 落地起点），与 `--from-instance` 互斥
- 新增 `RustScaffoldGenerator`（`schema` 特性，`confers::schema` 门面导出）：完整映射规则见其 rustdoc——非 required 字段 → `Option<T>` + `#[serde(default)]`；标量 `default` → 具体类型 + 生成的 `_default_*` 默认值函数；字符串枚举 → Rust 枚举（`#[serde(rename)]` 保原值）；内联嵌套对象提升为具名 struct；`$defs`/内部 `$ref` → 独立类型引用；自由形态对象 → `serde_json::Value`；`additionalProperties` 子模式 → `HashMap<String, T>`；属性名规范化 snake_case（`#[serde(rename)]` 保留原键、Rust 关键字用 raw identifier）
- fail-loud 契约：`oneOf`/`anyOf`/`allOf`/`not`（属性层与 `$defs` 命名层一致）、外部 `$ref`、解析不到 `$defs` 条目的内部 `$ref`、`patternProperties`、元组数组、混合非空类型显性报错并指明字段路径，绝不静默生成错误代码；复合 `default` 以 `TODO` 注释标注
- 生成安全与健壮性：进入生成代码的全部 schema 字符串（rename 值、枚举原值、字符串 `default`）做字面量转义（引号/反斜杠/控制字符），不可信 schema 无法注入可编译代码；文档注释与 `format` 提示中的控制字符可见化为 `\u{...}` 文本；错误消息同样过滤控制字符
- 标识符契约：规范化后非法的属性名（空、数字开头、归一化为全下划线——含单 CJK 字符等仅落入分隔符分支的名称→`_`，Rust 保留标识符无法逃逸）与不可 raw 化的关键字（`self`/`super`/`crate`）显性报错；归一化碰撞（`userName`/`user-name`、枚举变体 `a-b`/`a_b`）加数字后缀并保留原值 `#[serde(rename)]`
- 类型语义修正：字符串枚举 + 标量 `default` 生成匹配变体路径（`Enum::Variant`），default 不在变体内则报错；复合 `default` 的非 required 字段保持 `Option<T>` + `#[serde(default)]`（schema 可选语义）；nullable 类型数组的非 required 字段单层 `Option`（不再双重包装）；`$defs` 自由形态类型不再丢失 `HashMap` 导入；产物以单个换行结尾
- 契约测试：新增「生成产物必须通过 rustc 编译」冒烟测试（覆盖枚举×default、碰撞、转义、关键字属性、HashMap 导入组合）与注入回归测试
- 新增示例 `schema_to_rust`；新增 20 个生成器映射契约单测与 4 个 CLI 端到端测试；USER_GUIDE 补 `schema` 命令小节

### 远程来源转正（Beta → 稳定）

- **接口冻结承诺**：`remote` / `etcd` / `consul` / `etcd-watch` 的公开 API 自此冻结——1.0 前仅增量演进（只新增、不破坏），破坏性变更仅随 1.0 major 发布。冻结清单含 rc.6 全部新接口：
  - HTTP 轮询：`HttpPolledSourceBuilder::stale_on_error`（默认关闭，保持 fail-loud）、认证头注入 API、默认超时
  - etcd：`EtcdSourceBuilder` 全部 builder 方法（`endpoints` / `endpoint` / `username` / `password` / `prefix` / `format` / `interval` / `tls` / `operation_timeout` / `circuit_breaker_threshold` / `build`）、`EtcdTlsConfig`、`EtcdSource::watch_transport`
  - Consul：`ConsulSourceBuilder` 及其 builder 方法、`ConsulTlsConfig`
  - etcd watch：`WatchEventSource`、`EtcdWatcher`（`new` / `with_retry` / `last_revision` / `run`）、`EtcdWatchEvent`（`key` / `value` / `mod_revision`）、`EtcdWatchRetry`（`base` / `max`；默认 500ms 基础、30s 封顶的指数退避）、`EtcdWatchCallback`、`EtcdGrpcWatchSource`
- **新增**：`confers::async_trait` re-export（`remote` 特性门控）——`WatchEventSource` 以 async-trait 宏展开，外部实现该 trait 必须使用同一宏，re-export 即官方实现路径
- **运行时契约显性化**：回调同步内联调用、必须非阻塞（长任务请自行 spawn）写入 `EtcdWatchCallback`/`EtcdWatcher::run` rustdoc；重连后恢复点为映射即消费（批尾 at-most-once 间隙，需对账的消费者可对照 `last_revision`）；不可解码 UTF-8 的 etcd 值以 lossy（`U+FFFD`）交付而非报错重投——消除单一毒事件引发的无限重连风暴；流错误与建连失败增加 tracing::warn（`tracing` 特性门控）与 metrics 计数
- **兼容性守护**：新增 `tests/remote/etcd_watch.rs` watch 兼容性回归测试——外部 mock 实现驱动完整 watch 生命周期（事件按序交付、断流重连、`last_revision` 跨连接单调）、删除事件 `value: None` 穿透、冻结类型形状与默认退避语义
- **验证方式**：以现有测试矩阵替代长时 soak——六源 feature 门控（HTTP 轮询 / etcd / etcd-watch / consul / k8s / nacos）编译验证 + HTTP、etcd、Consul、watch 的 mock 联测 + watch 兼容性回归。真实集群集成测试（`tests/remote/etcd.rs` / `consul.rs`，依赖 `docker-compose.test.yml`）与长 soak 在无容器镜像仓库访问的沙箱环境无法执行，属环境限制而非代码缺陷
- **基线与门禁**：新增 `etcd_watch_bench`（整批 1000 事件分发 [83.8 µs, 84.8 µs, 86.0 µs]，约 0.085 µs/事件）；CI benchmarks job 特性扩为 `dev,etcd-watch,change-stream`，watch 相关基准（含既有 `watch_callback_bench`）自此随 CI 重测

## [0.6.0-rc.6] — 2026-09-28

### fix-audit-defects-r1（2026-09-23 审计缺陷修复，52 项）

#### 新增

- `#[config(profile)]` 落地：`RUN_ENV`（或 `profile_env` 指定变量）设置时自动加载 `<stem>.<env>.<ext>` 环境专属文件叠加
- `encrypt` 字段属性真实化：加载管线对统一 envelope 密文自动解密注入；支持 `ConfigBuilder::master_key` 显式注入主密钥（覆盖 `CONFERS_MASTER_KEY`）；Vault 登录按 `lease_duration` 提前 10% 主动刷新（403 反应式重登保留为兜底）
- `enc:v1:<keyver>:<payload>` 统一 envelope（旧格式兼容读）；审计链支持外置 HMAC 密钥（`AuditConfig::hmac_key`）
- `HttpPolledSource::stale_on_error` 选项（默认关闭，保持 fail-loud）、认证头注入 API、默认超时
- CLI `inspect`/`get` 默认脱敏 + `--reveal` 显式明文（stderr 警告）
- 公开 `PathValidator`（crate 根，无特性门控）与 `sensitive_names` 敏感名判定单一来源
- CLI 新增 `snapshot restore` 子命令（恢复最近或指定快照）；构建失败路径也会写快照供恢复
- `ConfigBuilder::env_separator`/`sensitive_paths` 方法；env 类型错误经 serde-path-to-error 携带字段路径
- 观测性：事件通道丢弃计数、`confers.env.path_conflict_dropped` / `confers.interpolation.sensitive_reference` telemetry 事件、`ProgressiveReloader::peek_candidate`

#### 变更（破坏性）

- 同优先级配置源按**声明顺序**合并（此前按 source_id 字母序）；默认值来源恒定最低优先级
- 带前缀的 env 变量路径冲突（标量 vs 嵌套）返回错误而非静默丢值；无前缀源确定性「嵌套形状胜出」+ 事件
- `load_file`/`load_file_with_env` 为全部 `#[config(default)]` 字段注册默认值；`load_file_with_env` 不再注入无前缀全进程环境
- `derive_field_key`/`decrypt`/`fetch_and_register` 等返回受管类型（`Zeroizing`/`SecretBytes`）；`SecureString::masked()` 与 `mask_value` 改定长掩码
- 宏 `#[config(dynamic)]` 的 `*_handle()` 单例化；`#[config(merge_strategy)]` 字段策略真实生效；null 不再覆盖已有值（全层级一致）
- `user_message()` 全路径过脱敏；CLI `get`/`inspect` 默认掩码
- 敏感文件（keys.json/快照/审计日志）落盘 0600

#### 修复

- 单文件 FsWatcher 原子替换后热重载静默失效；事件通道满静默丢事件；回调 panic 杀死重载循环
- Consul 空数组（KV 删除）永久续命旧配置；K8s in-cluster 缺 CA/超时；Nacos 无认证与熔断误报；Redis 总线断线无重连；总线版本号重启重置
- SSRF 黑名单补 `0.0.0.0/8` 等；弱密钥（全零等）拒绝；Vault token 过期自动重登；`rotate_master_key` 验证旧密钥
- `verify_audit_chain` 常量时间比较；冲突报告值脱敏；审计 sink 收到脱敏事件

#### 移除

- 死依赖 `secrecy`、`aes-gcm`；`KeyCachePolicy` 由死类型改为真实接线（Vault token 缓存按 NoCache/CacheWithTtl/CacheIndefinitely 生效）

### 变更（破坏面声明）

- **`#[config(validate)]` 保持兼容 no-op**：校验辅助方法 `confers_validate()` 改由新增的 opt-in 属性 `#[config(validate_helper)]` 生成（要求 `validation` feature 与 `#[derive(garde::Validate)]`）；两种属性都不会把校验自动挂进加载管线，旧代码（仅设置 `validate`）升级后行为与编译结果不变
- **`CorsValidator` 不再支持 unit-struct 裸名构造**（`let v: CorsValidator = CorsValidator;` 编译失败）：为支持 `with_origins_key` / `with_methods_key` / `with_max_age_key` 自定义键构造器而字段化；`CorsValidator::new()` 与 `Default` 行为不变（默认键名与旧版逐字节一致），经检索 confers / mnemis / sdforge 生态均仅以 `::new()` 构造，无实际消费者受影响
- **`WatcherGuard::shutdown` 异常路径行为变更**：被等待的任务 panic 时（join 返回 `Err`）现在记录错误并返回 `Ok(false)`；旧版把 panic 误报为 `Ok(true)`；超时与干净完成的语义不变
- **重载失败 reason 的日志注入防护**：`ReloadRolledBack` / `ReloadRejected` 及重载日志、canary 事件流中的 reason 在含控制字符或超过 200 字符时经 `flatten_reason` 清洗截断（控制符替换为空格、超限追加省略号）；正常输入逐字节不变

### 新增

- `WatcherGuard::with_task` / `set_task_handle` 由 `pub(crate)` 放宽为 `pub`；`shutdown` 文档注明超时返回 `Ok(false)` 的降级语义（任务不会被取消，仍在后台运行）
- `ConfigBuilder::env_source(EnvSource)`：接收预构建的 `EnvSource`（与 `env_prefix` / `env_separator` 的组合语义见 rustdoc）
- `hot-reload-kit` feature（默认关，不入 recommended/production/full 预置）+ `HotReloader` 门面：FsWatcher + 渐进重载 + watch 广播 + 优雅停机的装配形态；loader 经 spawn_blocking 卸载，panic 计入失败计数
- `ProgressiveReloader::with_pre_commit_check` + `PreCommitCheck` trait（`Err` 即拒）与新错误变体 `ConfigError::ReloadRejected`（受 `#[non_exhaustive]` 保护）
- `RemappedConfigProvider` 键重映射视图 + `JwtSecretValidator::with_secret_key` / `CorsValidator::with_{origins,methods,max_age}_key`（默认键名不变；CORS 三键仅部分重映射会触发既有规则的稳定误报，详见 `CorsValidator` rustdoc，建议仅 `JwtSecretValidator` 起步）

---

## [0.6.0-rc.5] — 2026-09-21

仅版本号发布（`Cargo.toml`/`Cargo.lock` bump）；包含 rc.4 之后的维护性变更：

- **i18n**：自研 i18n 门面（`src/i18n` catalog/locale，零外部依赖），为 `ConfigError` 实现 `LocalizedMsg` 并补全变体守卫测试
- **特性**：`keyring` / `cloud-kms` 隐含 `encryption`，引入 `async-core` 聚合谓词
- **工程**：接入 pre-commit 门禁与 detect-secrets 基线；typos 词表白名单；为 path-only 依赖补全 `version` 字段
- **测试**：NATS 集成测试名加纳秒熵防跨进程撞车；示例/文档/测试残留 `AppConfig` 衍生名统一为 `Confers` 前缀
- **依赖**：移除主 crate 直接依赖 `compact_str`（短字符串驻留从未实际接线；仅 `[workspace.dependencies]` 余留未继承的版本声明）

## [0.6.0-rc.4] — 2026-09-13

仅版本号发布（`Cargo.toml`/`Cargo.lock` bump，移除本地开发依赖口径、对齐 crates.io 发布链），无 API 与行为变更；同批收敛 CI 质量门禁（clippy 18 处、rustdoc 私有项链接、rustfmt 对齐）。

---

## [0.6.0-rc.3] — 2026-09-10

> 本节包含 `workspace-rc3-hardening` 与 `workspace-rc4-completion` 两批内容（rc.3 发布前累计；版本规则：目标 = crates.io 已发布 rc.2 + 1）。
> 版本号 0.6.0-rc.3 已跳过、未发布（无 tag、未上 crates.io），本节内容随 0.6.0-rc.4 一并发布。

### 新增（workspace-rc4-completion）

- **统一变更流端口**（`change-stream` feature）：`ChangeStream` trait（publish/subscribe/ack），`InMemoryChangeStream` 复用 `ConfigBus`；watch 与 remote 变更统一发布 `ChangeEvent` 信封（key/旧值/新值/来源 + 单调版本）。
- **宏四属性 codegen 真实消费**：`flatten`（顶层键提升进嵌套结构，`ConfigFieldKeys` 钩子）、`dynamic`（`<field>_handle()` DynamicField 运行时句柄）、`interpolate`（字段值 `${key}`/`${key:default}` 按合并树解析）、`watch`（字段级热重载订阅器 `field_watcher`）；四属性组合不冲突，macro_e2e 语义断言。
- **etcd 原生 watch 流**（`etcd-watch` feature）：`EtcdWatcher` 消费 watch 流替代前缀 GET 轮询，断线有界指数退避自动重连，`WatchEventSource` 抽象可 mock；`EtcdSource::watch_transport()` 一键接入。
- **Kubernetes 配置源**（`k8s` feature）：`K8sMountedSource` 挂载卷源（kubelet 原子写 symlink swap 感知，`..data` 代际检测）+ `K8sApiSource` REST API 源骨架（ConfigMap/Secret、in-cluster 探测、Secret base64 解码）。
- **Nacos 配置源**（`nacos` feature）：HTTP Open API 拉取 + 定时监听（未变更内容命中缓存快照），命名空间/分组映射，熔断器保护。
- **CLI `doctor` 子命令**：schema 结构 / 来源优先级链 / 加密字段可解密（`enc:v1:` 信封 + `CONFERS_MASTER_KEY`）/ env 覆盖冲突 / 加载五项检查；单行 JSON 报告 + 退出码 0 健康 / 1 警告 / 2 错误。
- **CLI `schema --from-instance`**：从已加载配置实例反推 JSON Schema 草稿（对象 required、数组 items、标量类型推断）。
- **tracing 集成**（`tracing` feature）：load/reload/decrypt/remote fetch 四条关键路径 span 与事件；与 MetricsBackend 并行不互斥；feature 关闭零开销。
- **AuditSink 多 sink 端口**：对象安全 `pub trait AuditSink: Send + Sync`，`AuditWriter` 支持注入多个 sink（`add_sink`/`with_sink`/builder），默认本地 HMAC 链文件保留不变；供 inklog 上层实现。
- **性能基线门禁**：`watch_callback_bench`（变更流往返/扇出/记账）+ load/merge/hot_path 基线数字记录 `docs/PERFORMANCE.md`。
- **Agent 知识包**：CLI `docs --agent` 输出子命令/参数/退出码契约/常见任务配方（JSON + Markdown，≤200 行）。
- **宏批量重命名**：`#[config(rename_all = "camelCase|snake_case|kebab-case")]`，codegen 在反序列化前把外部键映射回 serde 字段名（serde 名显式出现时优先），非法风格宏展开期报错（trybuild）。
- **云密钥后端**（`cloud-kms` feature）：Vault transit KeyProvider MVP（`POST /v1/transit/decrypt/{key}` 解包 32 字节主密钥），`CloudKmsBackend` 端口留 AWS/GCP 扩展点；mock server 测试。
- **Vault 认证增强**：`VaultAuth::AppRole` / `Kubernetes`（pod JWT + auth role）登录换 client token 并缓存；`kubernetes_from_service_account` 从 in-pod token 文件构建；mock 测试。
- **系统钥匙串**（`keyring` feature）：`KeyringStore` 端口 + `SecretToolKeyringStore`（freedesktop Secret Service）/ `FileKeyringStore`（chmod-600 回退并告警）；`MasterKeyStore::from_environment` 自动选择；无 DBus 环境 skip 测试。
- **OpenFeature 灰度引擎**（`openfeature` feature）：`FeatureProvider` 端口 + `NoOpProvider` + `StaticFlagProvider`（属性规则 + 按 targeting key 确定性百分比分桶）+ `ToggleRegistryProvider` 桥接既有 FeatureToggleRegistry。
- **总线版本仲裁**：`VersionArbitratedBus` 包装任意 `ConfigBus`，发布端按实例打单调版本，订阅端丢弃乱序/过期/重复版本并计数；未版本化事件 fail-open。
- **零拷贝热路径**：`InMemoryConfig` 内部存储改 `Arc<AnnotatedValue>`，新增 `SharedValueReader::get_shared()` 共享句柄读取；10 KiB value 读取 ~2.1x 提升（bench 对比记录 PERFORMANCE.md）。
- **金丝雀联动**（`change-stream` feature）：ProgressiveReloader 阶段迁移（trial_started/committed/rolled_back）发布 `ChangeSource::Canary` 事件到 ChangeStream，供上层编排。
- **惰性分段解析**（`lazy-parse` feature）：`LazySegmentedConfig` 按顶层 TOML 表头切分（纯行扫描），段首次访问才解析并缓存；单测断言未访问段零解析。

### 变更（workspace-rc4-completion）

- `InMemoryConfig` 值存储改为 `Arc<AnnotatedValue>`（公共 API 兼容；`get_raw`/`get_string` 语义不变）。

### 修复（workspace-rc3-hardening）

- **宏 env 键名覆盖**：修复 `#[confers(name_env = "X")]` 生成的 env 键与声明不一致的缺陷，确保自定义 env 键名与默认命名规则互不污染。
- **skip+default 组合**：修复 `skip` 字段参与加载且 `default` 被 env/文件覆盖的缺陷，修正字段过滤顺序。

### 新增（workspace-rc3-hardening）

- **审计 HMAC 链式签名**（`audit` feature）：`AuditEvent` 落盘前计算 `HMAC-SHA256(prev_hash || canonical_event_bytes)`，链首用随机 salt；审计文件写入链头/链尾元数据；提供 `verify_audit_chain(path)` 校验函数。
- **MetricsBackend 关键路径埋点**：loader 加载完成/失败、watcher 触发次数、remote 源拉取延迟与错误、secret 解密错误四类路径接入 `MetricsBackend`，指标名前缀 `confers_`。
- **CLI `schema` 子命令**：输出配置类型的 JSON Schema（需 `cli` feature，自动启用 `schema`）。
- **CLI `get <key>` 子命令**：按点分路径获取配置值，输出单行稳定 JSON；缺失键返回 `null`（退出码 0）。
- **CLI `--fields` 全局选项**：裁剪 JSON 输出到指定字段（逗号分隔点分路径）。
- **CLI 退出码契约**：0 成功 / 1 配置错误 / 2 I/O 错误。

---

## [0.6.0-rc.2] - 2026-09-07

### Changed

- 依赖升级：12+ 项 0.x 依赖刷新（async-nats 0.50、base64 0.23、chacha20poly1305 0.11、aead 0.6（适配 getrandom 0.4）、compact_str 0.10、garde 0.23、hkdf 0.13、sha2 0.11、aes-gcm 0.11、serde_ini 0.2、etcd-client 0.20、rand 0.10、criterion 0.8）+ darling 0.24 / syn 3.0 工具链 + indexmap/ipnet/redis 刷新
- 版本号递增至 `0.6.0-rc.2`（下一个 minor 预发布）

### 测试

- 测试金字塔 + E2E 固化：补齐 7 个缺失 e2e 测试文件并 `[[test]]` 注册；NATS 容器集成测试容器化验证（测毕零残留）；docs/TEST_SCENARIOS.md 场景固化

### 文档

- 安装示例版本统一 0.6.0-rc.2（10 处）；MSRV 文字声明对齐 1.97.1；SECURITY 文档 CHANGELOG 断链修复

---

## [0.5.1] - 2026-08-06

### 新增

- **安全规则**：新增 `SecurityValidator` trait 与内置校验器（JWT 密钥强度、CORS 配置、SSRF 防护、TLS 设置）。
- **安全校验器注册表**：`SecurityValidatorRegistry` 及 `with_defaults()`，开箱即用的安全检查。
- **特性开关**：基于 `DashMap` 的运行时 `FeatureToggleRegistry`，线程安全的并发开关管理。
- **特性开关配置加载**：`load_from_config()` 方法，可从 `ConfigProvider` 填充开关状态。

### 修复

- **SSRF 白名单绕过**：`starts_with` 前缀检查替换为正确的 URL 边界匹配，防止 `https://127.0.0.1.evil.com` 式绕过。
- **TLS 版本比较**：字典序字符串比较替换为整数元组比较（修复 `"1.12" < "1.2"` 这类误报）。
- **CORS 重复违规**：`max_age` 校验器改用 `else if`，避免同一配置值产生重复违规。
- **CORS 负整数回绕**：新增 `age >= 0` 守卫，防止负 `i64` → `u64` 溢出。
- **正则静默编译**：非法的自定义 pattern 现在返回错误，而不是被静默丢弃。
- **弃用 API 迁移**：安全规则代码中的全部 `as_string()` 调用替换为 `as_str()`。
- **IPv6 括号边界情况**：格式非法的 IPv6 地址（缺少右括号）现在报告 Warning，而不是静默通过。
- **环境变量映射校验**：`validate_env_mapping` 现在同时校验键与值。

### 变更

- **性能**：JWT 弱密钥查找从 `O(n)` 线性扫描改为 `O(1)` 的 `HashSet` 查找。
- **性能**：TLS 加密套件比较将冗余的 `eq_ignore_ascii_case` 简化为直接 `==`（两侧本已为大写）。

---

## [0.5.0] - 2026-08-04

### 变更

- **破坏性 - ConfigBuilder**：移除 6 个无用方法：`watch()`、`validate()`、`key_provider()`、`metrics()`、`reload_strategy()`、`build_timeout()`；同时移除 `ReloadStrategy` 枚举。
- **破坏性 - KeyManager**：`new(path: PathBuf)` 简化为 `new()`（无参数）。
- **破坏性 - AuditConfig/AuditWriterBuilder**：移除 `durable_wal()` 与 `channel_size()` 方法及对应字段。
- **破坏性 - AuditWriter**：`write()` 与 `log_*()` 方法现在返回 `ConfigResult<()>`，不再返回 `()`。
- **破坏性 - TypeScriptGenerator**：`generate()` 现在返回 `ConfigResult<String>`，不再返回 `String`。
- **破坏性 - CryptoError**：移除 `InvalidKeyLength(0)` 变体，新增 `KeyNotFound` 变体。
- **破坏性 - MergeStrategy**：`Custom` 变体的 `PartialEq` 现在只按名称比较（不再比较函数指针）。

### 新增

- **熔断器**：新增 `CircuitBreaker` 与 `CircuitState`，提升远程来源的韧性。
- **密钥熵校验**：密钥管理器增加熵值校验。
- **错误信息脱敏**：防止敏感配置值泄露到错误输出。

### 修复

- NATS JetStream 测试偶发失败（为并行测试添加 `#[serial]`）。
- Clippy 警告：将字面量布尔值的 `assert_eq!` 替换掉，修复近似常量警告。
- `.env` 解析错误现在显式上报，不再被静默丢弃。
- 插值自引用默认值不再误报循环引用。
- 安全模块的 `allowed_patterns` 现在正确生效。
- 修复远程来源 `source_id` 的一致性问题。
- `cleanup_old_keys` 改为按版本号过滤，而不是按 Active 状态。
- 移除库运行时代码中的全部 `eprintln` 副作用。

### 变更

- **性能**：合并引擎的 owned-path 优化。

---

## [0.4.0] - 2026-07-03

### 变更

- **破坏性 - SecureString 不再实现 `Clone`**：移除 `impl Clone for SecureString`，与既定安全姿态保持一致（"防止克隆：禁止 Clone"）。这防止敏感材料在内存中被无意复制。需要跨线程共享 `SecureString` 的调用方应改用 `Arc<SecureString>`。

### 修复

- **插值嵌套默认值解析（M1）**：`:-` 默认值分隔符现在通过一个可感知 `${}` 嵌套深度的解析器识别。此前 `${outer:${inner:-fallback}}` 会在内层 `:-` 处被错误切分，破坏嵌套默认值。
- **URL 校验误杀查询串中的 & 符号（M7）**：`&` 字符已从 shell 注入危险字符正则中移除。它是标准的 URL 查询串分隔符（`?key=val&key2=val2`）。shell 逻辑运算符 `&&` 与 `||` 仍由独立模式捕获。
- **`load_env_file` 无上限内存消耗（M9）**：为 `load_env_file` 添加 `MAX_ENV_FILE_SIZE`（1 MiB）与 `MAX_ENV_LINE_LENGTH`（16 KiB）检查。超限文件在进入内存前即被拒绝，并附带说明性错误。

### 新增

- **错误类型分离**：将错误拆分为配置阶段（`ConfigConfigError`）与运行时阶段（`ConfersError`）。
- **工厂函数**：新增 `new_in_memory_validated()`，为 BrickArchitecture 快速失败式初始化返回 `Result`（后移除，改用 `InMemoryConfig::new_validated()`）。
- **向后兼容**：新增别名 `ConfersError = ConfigError`、`ConfersResult<T>` 以保护既有代码。
- **SECURITY.md**：新增全面的安全策略文档，涵盖漏洞报告流程、安全特性说明、最佳实践指南与安全审计流程。
- **ADR-041**：async trait 稳定化策略 —— 评估从 `async-trait` crate 迁移到原生 `async fn in traits`。
- **ADR-042**：来源注册机制 —— 统一的 `SourceProvider` trait 与 `SourceRegistry`，支持可扩展的数据源注册。
- **ADR-043**：错误类型简化策略 —— 分层错误架构与简化后的公开 API。
- **ADR-044**：测试覆盖率目标（80%）—— 按模块的覆盖率要求与 CI 集成。
- **ADR-045**：API 版本化策略 —— semver 强制、弃用流程与破坏性变更政策。
- **BrickArchitecture 合规**：配置阶段错误类型 `ConfigConfigError`，含 10 个变体与错误码（2001-2999）。

### 变更

- **文档 - README.md**：新增完整的功能矩阵表与示例目录章节，直接链接到全部 13 个可运行示例。
- **文档 - CONTRIBUTING.md**：新增明确的代码覆盖率要求（>= 80%），并按 ADR-044 更新测试指南。
- **文档 - lib.rs**：新增 BrickArchitecture 错误分离章节与迁移示例。
- **安全**：安全文档现覆盖全部加密、输入校验、SSRF 防护、审计日志与密钥管理特性。
- **测试质量 - T-C-1**：审计了 `tests/` 中 61 处弱断言模式。强化了其中 3 处确实薄弱的断言：现在会验证被丢弃的 `save()` 返回值；将 save+list 的裸 `is_ok()` 替换为文件存在性与快照计数检查；将被丢弃的 `build()` 结果替换为显式断言，验证 TLS 配置可被接受。

---

## [0.3.0] - 2026-03-04

### 🎉 新增

#### TypeScript Schema 生成

- 新增 `typescript-schema` 特性
- 支持从 Rust 类型生成 TypeScript 类型定义
- API：`confers::schema::generate_typescript::<T>()`

#### 密钥管理系统（key 特性）

- `KeyManager`：密钥生命周期管理
- `KeyStorage`：加密密钥存储（XChaCha20）
- `KeyRotationService`：自动密钥轮换
- 密钥元数据与版本管理

#### 安全模块（security 特性）

- `EnvSecurityValidator`：环境变量安全校验
- `ErrorSanitizer`：错误信息中的敏感数据脱敏
- `ConfigInjector`：安全配置注入
- `SecureString`：自动清零的安全字符串

#### 并行校验

- 提升大型配置的校验性能

### 🔧 变更

- 更新特性预设定义（minimal、recommended、dev、production、full、distributed）
- 优化 XChaCha20 加密性能
- 提升错误信息的可读性
- **文档与代码库同步**：
  - 全部文档中的版本号从 0.2.2 更新到 0.3.0
  - 修正 API 方法名：`load()` → `load_sync()`
  - 移除不存在的 `ConfersCli` API 引用
  - 移除不存在的远程 builder 方法（`with_remote_config` 等）
  - 远程配置示例替换为 `source()` 方法写法
  - 更新特性标志列表以匹配 Cargo.toml
  - 补充缺失的特性文档（config-bus、progressive-reload 等）

### 🐛 修复

- 修复 `progressive-reload` 特性门控（缺少 `async_trait`）
- 修复 `encryption` 特性测试 panic

### 🔒 安全

- 密钥管理使用 HKDF 密钥派生
- 增强环境变量注入防护
- 新增路径穿越防护

### 📊 统计

- 新增文件：17
- 新增代码：6,855 行
- 新增特性：4 个（typescript-schema、security、key、parallel）

## [0.2.2] - 2026-01-25

### 安全

- 内部函数可见性加固（内部辅助函数改为 pub(crate)）
- TlsConfig 重构为 builder 模式
- 应用 Clippy 代码风格修复

### 变更

- 在整个代码库中统一 FileFormat
- 测试：全部 167 个单元测试通过
- 测试：全部 34 个文档测试通过
- 测试：cargo clippy --all-features 通过
- 测试：cargo deny check 通过

## [0.2.1] - 2026-01-17

### 安全

- **增强的审计日志系统**：新增完善的审计日志，含事件分类、完整性保护、日志轮转与查询能力
- **增强的配置校验**：实现高级校验系统，含范围、依赖、格式与一致性校验器
- **安全文档**：在 API 参考与用户指南中新增全面的安全文档
- **安全注解**：为敏感 API 方法（加密、密钥管理、审计日志、配置校验）添加安全注解
- **安全示例**：新增审计日志、配置校验与密钥管理的安全示例

### 新增

- 审计事件类型与优先级（ConfigLoad、KeyRotation、SecurityViolation 等）
- 带元数据追踪的审计事件生成器
- 带 HMAC 完整性保护的审计日志写入器
- 带 gzip 压缩的日志轮转与归档
- 支持过滤与分页的审计日志查询接口
- 用于可扩展校验的 AdvancedConfigValidator trait
- 按优先级执行校验器的 ValidationEngine
- 数值范围校验的 RangeFieldValidator
- 字段依赖校验的 DependencyValidator
- 字符串格式校验的 FormatValidator（邮箱、URL 等）
- 跨字段一致性校验的 ConsistencyValidator
- 带 LRU 缓存提升性能的 CachedValidationEngine
- API_REFERENCE.md 中的全面安全文档
- USER_GUIDE.md 中的安全配置最佳实践
- examples/src/06-encryption/ 与 examples/src/08-audit/ 中的安全示例
- examples/src/02-validation/02-validation-advanced_validation.rs 中的高级校验示例

### 变更

- 在 ConfigError 枚举中新增 EncryptionError 与 DecryptionError
- 在 src/audit/mod.rs 中新增 std::io::Write 导入
- 修复 src/core/loader.rs 中 SecureString 的 Arc::clone 用法
- 更新 Cargo.toml，新增用于日志压缩的 flate2 依赖
- 更新 API_REFERENCE.md，补充全面的安全说明
- 更新 USER_GUIDE.md，补充安全配置最佳实践
- 更新示例，加入完整的安全演示

### 修复

- 修复 `Config` 派生宏中 `Arc<SecureString>` 类型标注导致的编译错误
- 修复 src/audit/mod.rs 与 src/validator/mod.rs 中未使用导入的警告
- 修复带正确导入与类型的文档测试

- 测试：全部 204 个测试通过（178 单元测试 + 26 文档测试）
- 测试：全部安全测试通过（93 个）
- 测试：全部审计测试通过（6 个）
- 测试：全部校验器测试通过（13 个）

## [0.2.0] - 2026-01-16

### 安全

- **内部实现保护**：将 `RemoteConfig` 与 `ConfigLoader` 中的敏感字段私有化，防止意外暴露。
- **敏感数据隔离**：将 `HttpProvider` 与 `RemoteConfig` 中敏感字段（`password`、`token`、`bearer_token`）的 `String` 替换为 `Arc<SecureString>`。
- **访问控制**：为 `EnvironmentValidationConfig` 与 `RemoteConfig` 引入安全的 Builder 模式，强制通过 `with_auth_secure` 与 `with_bearer_token_secure` 安全构造。
- **SSRF 防护**：增强 `HttpProvider`，在所有加载方法（`load`、`load_sync`）中校验 URL，即使内部状态被修改也能防止潜在 SSRF 攻击。
- **泄露防护**：修复 HTTP 提供方在请求认证期间正确处理 `SecureString`，避免潜在的敏感数据泄露（避免脱敏输出）。
- 新增生产级安全模块：
  - 带自动内存清零的 SecureString，保护敏感数据
  - 用于安全运行时配置注入的 ConfigInjector
  - 防 SQL/命令注入的 InputValidator
  - 错误信息敏感数据脱敏的 ErrorSanitizer
- 在过程宏中新增敏感数据检测与告警：
  - 在编译期检测硬编码的密码、令牌与私钥
  - 发出运行时警告，引导用户使用更安全的替代方案
  - 实现输入长度限制，防止 DoS 攻击
- 修复 SSRF 测试模式绕过 —— 仅允许在非生产环境绕过 localhost 限制
- 修复环境变量注入 —— 替换前增加校验
- 完善路径穿越防护：
  - 检测包括 URL 编码与 Windows 路径在内的穿越模式
  - 屏蔽对敏感系统目录（/etc、/usr、/var/log 等）的访问
  - 通过规范化（canonicalization）防止符号链接攻击
- 增强安全配置注入器的校验能力
- 重构错误脱敏以提升安全性
- 改进输入校验逻辑

### 修复

- 修复禁用校验特性时 `Config` 派生宏的编译错误：自动实现 `OptionalValidate` trait。
- 解决 `ConfigLoader` 中方法重复定义的问题。

### 新增

- 创建统一的文件格式探测模块（消除 4 处重复实现）
- 新增全面的安全测试（800+ 行测试覆盖）

### 变更

- 用完整的错误类型增强错误处理
- 优化远程配置的 HTTP 提供方
- 将 `resource/` 目录重命名为 `docs/image/` 以便更好组织
- 更新文档样式与链接
- 减少约 120 行重复代码
- 将格式探测逻辑集中到 utils/file_format.rs
- **破坏性**：默认特性从 `["derive", "validation", "cli"]` 变更为 `["derive"]`，最小化依赖足迹
- 将 `rustls` 改为可选（现在仅随 `remote` 特性启用）
- 将 `chrono`、`sysinfo`、`lru` 改为可选依赖（移入 `encryption` 与 `monitoring` 特性）
- 移除未使用的 `num_cpus` 依赖
- 新增特性预设，简化配置：
  - `minimal` - 仅配置加载
  - `recommended` - 配置加载 + 校验
  - `dev` - 全工具开发配置
  - `production` - 生产就绪配置
  - `full` - 启用全部特性
- 为全部可选特性添加条件编译，最小化编译时间与二进制体积
- 更新 `remote` 特性，纳入 `rustls`、`rustls-pki-types`、`tokio-rustls`、`failsafe` 与 `base64`
- 更新 `encryption` 特性，纳入 `lru` 与 `chrono`
- 修复 `RefreshKind::new()` 为 `RefreshKind::nothing()` 以兼容 sysinfo

### 依赖更新

- 全部依赖更新至最新稳定版本
- `lru` 从 0.12 升级到 0.16.3，修复健全性问题（RUSTSEC-2026-0002）
- 更新核心依赖：tokio 1.48 → 1.49、serde、validator、schemars、thiserror、clap 等
- 依赖更新后全部 108 个测试通过

## [0.1.1] - 2026-01-02

### 安全

- 为 SSRF 校验新增 DNS 重绑定防护，防止通过主机名解析发起 SSRF 攻击
- 为 ConfigError 新增 safe_display() 方法，脱敏错误信息中的敏感内容
- 在错误信息中遮蔽密钥 ID，防止敏感数据泄露

### 修复

- 将默认内存上限从 10MB 提高到 512MB，避免生产环境故障
- 将 HTTP 请求超时改为可配置（默认 30s），提升性能可控性
- 将 RwLock 的 unwrap() 调用替换为恰当的错误处理，避免 panic
- 校验器注册表方法改为返回 Result，不再 panic

### 新增

- 新增 nonce 缓存监控方法（usage_percent、cache_stats），提升生产可观测性
- 在 ConfigLoader 中对过低的内存上限（< 100MB）增加警告

### 变更

- 简化 SSRF 校验中的布尔表达式（Clippy 改进）
- 改进代码格式与文档

## [0.1.0] - 2025-12-27

### 新增

- 基于派生宏的类型安全配置管理
- 多格式支持（TOML、YAML、JSON、INI）
- 环境变量覆盖支持
- 内置校验系统集成
- JSON Schema 生成
- 文件监听与热重载支持
- 敏感配置的加密存储
- 配置访问与变更的审计日志
- 远程配置支持（etcd、Consul、HTTP）
- 多命令 CLI 工具（encrypt、validate、diff、generate 等）

### 变更

- 首次发布
- 改进文档与示例

### 安全

- 安全内存清理
- 敏感数据 AES 加密
- PBKDF2 密钥派生

### 致谢

- 感谢所有贡献者与 Rust 社区
