# Tasks — fix-audit-defects-r1

- [x] [T001] [P0] 修复同优先级排序：`src/impl_/config/chain.rs` 排序键去掉 source_id 字母序改为稳定排序保持声明顺序，`src/impl_/config/source.rs` DefaultSource priority 改为 -100；新增测试断言「文件值覆盖 default」「后声明文件覆盖先声明」「memory 与 env 同优先级按声明顺序」，并删除 `tests/core/merge.rs` 中字母序绕过注释；运行受影响测试确认 green → src/impl_/config/chain.rs
- [x] [T002] [P0] 修复 EnvSource 嵌套冲突：`src/impl_/config/source.rs` 的 `insert_nested` 先对收集变量按名排序消除迭代顺序依赖，标量与嵌套路径前缀冲突时返回携带变量名与路径的 ConfigError 而非静默丢值；新增测试断言 `X_DB` + `X_DB_HOST` 共存时得到确定性错误且任意 env 顺序结果一致 → src/impl_/config/source.rs
- [x] [T003] [P0] 宏 `load_file_with_env` 去除无条件全量 `.env()`：`macros/src/codegen/load.rs` 生成的链只含显式声明 env 映射；新增测试断言 `deny_unknown_fields` 结构体在带无关环境变量（如 PATH/HOME）时构建成功、声明字段仍可被前缀变量覆盖 → macros/src/codegen/load.rs
- [x] [T004] [P0] 宏文件加载路径默认值生效：`macros/src/codegen/load.rs` 的 `load_file`/`load_file_with_env` 为全部含 `#[config(default)]` 字段生成默认注册（对齐 load_sync）；新增测试断言部分 TOML 缺字段时 `load_file` 用默认值构建成功不再报 missing field → macros/src/codegen/load.rs
- [x] [T005] [P1] 统一 null 覆盖语义：`src/impl_/merger/engine.rs` 的 `apply_leaf_strategy` 增加 Null 分支与根级一致（null 不覆盖已有值）；新增测试断言 map 内 null 高优先级不覆盖文件值且与根级行为一致 → src/impl_/merger/engine.rs
- [x] [T006] [P1] env 类型错误携带字段路径：修复 `src/impl_/config/builder.rs:333-337` 空 key，反序列化错误信息含完整字段路径；新增测试断言 String 字段遇 env 数值型值时错误信息含字段名 → src/impl_/config/builder.rs
- [x] [T007] [P1] env 嵌套 separator 可配置：`ConfigBuilder`/`SourceChainBuilder` 新增 `env_separator` 透传至 EnvSource；新增测试断言 `APP_DB__HOST` 经 `env_separator("__")` 映射到 `db.host`，USER_GUIDE 嵌套示例语义可用 → src/impl_/config/builder.rs
- [x] [T008] [P1] 接线 merge_strategy：`src/impl_/merger/engine.rs` 合并递归携带路径并查询字段策略映射（chain 层传入策略表），无策略路径行为不变；新增测试断言 Append/Replace 策略在嵌套字段真实生效 → src/impl_/merger/engine.rs
- [x] [T009] [P1] 实现 profile 叠加：`macros/src/codegen/load.rs` 对启用 profile 的结构体在基础文件后追加 `<stem>.<env>.<ext>`（env 取 profile_env 变量缺省 RUN_ENV，文件不存在跳过）；新增测试断言 prod 叠加文件覆盖基础文件字段 → macros/src/codegen/load.rs
- [x] [T010] [P2] MergeEngine priority 一致：`src/impl_/merger/engine.rs` 的 `merge` 按入口 priority 交换 low/high，`report_conflict` winner 与实际结构胜者一致；新增测试断言 priority=10 low 胜 priority=5 high 且报告 winner=Low → src/impl_/merger/engine.rs
- [x] [T011] [P2] 宏默认值边界：`macros/src/codegen/defaults.rs` 与 `load.rs` 对 `#[config(default)]` 空值形式生成 `<T as Default>::default().into()`、对 Option 字段 `default = None` 生成 `None`；新增单测覆盖两种写法编译与运行 → macros/src/codegen/defaults.rs
- [x] [T012] [P2] 宏敏感 `_FILE` 报错对齐：`macros/src/codegen/load.rs` 生成的敏感字段 `_FILE` 路径无效时返回错误而非静默跳过，且与 EnvSource 错误文案一致；新增测试断言坏路径触发错误 → macros/src/codegen/load.rs
- [x] [T013] [P2] 插值补强：`src/tree_transform.rs` 与 `macros/src/codegen/load.rs` 支持 `$${VAR}` 转义输出字面量、插值 key 使用 serde 名（serde(rename) 后仍生效）、嵌套结构体字段参与插值；新增三项测试 → src/tree_transform.rs
- [x] [T014] [P1] 插值敏感引用告警：加载路径在非敏感字段引用 `sensitive_vars` 时产生 tracing warning 并计入校验 warning 列表；新增测试断言 `public_url = "${API_KEY}"` 触发告警 → src/tree_transform.rs
- [x] [T015] [P0] 修复 FsWatcher 原子替换失聪：`src/watcher/fs_watcher.rs` 单文件模式改为监听父目录+精确路径过滤（对齐 MultiFsWatcher 的 rename 防护）；新增「连续两次 rename 原子替换均收到事件」回归测试 → src/watcher/fs_watcher.rs
- [x] [T016] [P0] 事件丢弃可观测：`src/watcher/fs_watcher.rs` 两处 `try_send` Full 分支加 `tracing::warn!` 与 `Arc<AtomicU64>` 丢弃计数并暴露 `dropped_events()`；新增测试模拟慢消费者断言计数增长 → src/watcher/fs_watcher.rs
- [x] [T017] [P0] 回调 panic 隔离：`src/impl_/dynamic.rs` 的 `DynamicField::update` 回调执行包 `catch_unwind(AssertUnwindSafe)`，panic 回调被计数吞掉且后续回调与 store 不受影响；新增测试断言 panic 回调后其余回调仍执行、update 正常返回 → src/impl_/dynamic.rs
- [x] [T018] [P0] dynamic handle 单例化：`macros/src/codegen/field_attrs.rs` 生成的 `*_handle()` 用 `OnceLock<Arc<DynamicField<_>>>` 返回共享实例；新增测试断言两次调用返回同一实例且 update 互通 → macros/src/codegen/field_attrs.rs
- [x] [T019] [P2] 并发 update 回调有序：`src/impl_/dynamic.rs` update 携带单调版本号，过期回调跳过使回调观测顺序等于最终存储值；新增多线程并发 update 测试断言最后观测值==最终存储值 → src/impl_/dynamic.rs
- [x] [T020] [P1] ChangeStream Lagged 显式化：`src/stream.rs` 驱逐推进 watermark，`get(version)` 命中已驱逐区间返回 `Lagged{from}` 错误，订阅流把 Lagged 转为显式 resync 事件不再被 filter_map 吞掉；新增测试 → src/stream.rs
- [x] [T021] [P1] AdaptiveDebouncer 接线：`src/watcher/fs_watcher.rs` 事件出口经 AdaptiveDebouncer 合并（利用其既有 CAS 回归测试），WAT-18（tests/e2e/watch_e2e.rs:128）去 `#[ignore]` 并确认 green；新增快速连写合并测试 → src/watcher/fs_watcher.rs
- [x] [T022] [P2] overrides 移除 TTL 静默回退：`src/impl_/default.rs:63-66` 缓存改永久条目（动态 set 值不再 5 分钟回退）；新增测试断言 set 后长时间读取仍为新值 → src/impl_/default.rs
- [x] [T023] [P1] 渐进发布接线：`src/watcher/progressive.rs` 新增 `peek_candidate()` 公开观测；`src/watcher/mod.rs` 的 `rollback_on_validation_failure` 在提交前校验失败路径真实消费；`src/impl_/migration.rs` 的 MigrationOnReload 在提交成功后调用 migration registry；新增三个行为测试 → src/watcher/progressive.rs
- [x] [T024] [P0] SSRF 黑名单补全：`src/remote/poll.rs` 补 `0.0.0.0/8`、IPv6 unspecified、NAT64 `64:ff9b::/96`，黑名单与 `src/security/rules/ssrf.rs` 收敛为单一来源函数；修正 poll.rs:934 固化 `::0 not blocked` 的测试；新增 `https://0.0.0.0:port` 与十进制 `https://0` 被拒测试 → src/remote/poll.rs
- [x] [T025] [P0] Consul 删除语义：`src/remote/consul.rs:420-454` recurse 空数组时推进 last_index 并返回空配置集（仅 X-Consul-Index 不变才视为无变化）；新增 mock 测试断言删除 KV 后 poll 返回空配置而非 cached → src/remote/consul.rs
- [x] [T026] [P1] K8s in-cluster 可用：`src/remote/k8s.rs` 加载 SA ca.crt 经 `add_root_certificate` 并应用默认超时（connect 10s/total 30s）；新增单测断言 client 构造含 CA 与超时配置 → src/remote/k8s.rs
- [x] [T027] [P1] HTTP 轮询源超时与认证：`src/remote/poll.rs` builder 未设置时应用默认超时，新增 `with_header` 注入认证头（值不进 Debug/日志）；新增测试断言默认超时存在与 header 生效 → src/remote/poll.rs
- [x] [T028] [P1] 容灾兜底：`src/remote/poll.rs` 新增 `stale_on_error(bool)`（默认 false 保持 fail-loud），开启后熔断打开/请求失败返回 cached 附 warning 标记；`src/impl_/config/builder.rs` 失败路径也写快照；`src/cli/mod.rs` 新增 `snapshot restore` 子命令；新增三项测试 → src/remote/poll.rs
- [x] [T029] [P0] Redis 总线重连：`src/bus/redis.rs:156-168` 订阅循环外包指数退避重连（1s→30s 封顶）+ 断连 tracing warn；新增 mock 断线重连单测 → src/bus/redis.rs
- [x] [T030] [P0] 总线版本 epoch：`src/bus/arbiter.rs` 事件携带发布方进程级随机 epoch，仲裁器按 `(publisher_id, epoch)` 分轨单调去重，未知 epoch 接受；新增测试断言发布方重建（新 epoch、seq 从 1）后事件不被丢弃 → src/bus/arbiter.rs
- [x] [T031] [P1] Nacos 认证与熔断误报：`src/remote/nacos.rs` 支持 username/password 登录换 accessToken 附加请求（401 重登一次）；`:222-259` 熔断 try_lock WouldBlock 记 unknown 不记失败；新增 mock 登录与争用测试 → src/remote/nacos.rs
- [x] [T032] [P2] 熔断器覆盖 etcd/Consul/K8s：三源接入 `src/remote/circuit_breaker.rs`（失败计数/打开/半开恢复）；新增每源熔断打开行为测试 → src/remote/consul.rs
- [x] [T033] [P2] NATS 毒消息上限：`src/bus/nats.rs` consumer 设 `max_deliver(5)`，反序列化失败 `Nak` 带 5s 延迟；新增单测断言配置生效 → src/bus/nats.rs
- [x] [T034] [P2] 快照原子写：`src/impl_/snapshot.rs:313,391` 改 tmp+rename 原子替换；新增测试断言写入期间崩溃不留半截文件（模拟：写 tmp 后断言目标不存在，rename 后存在） → src/impl_/snapshot.rs
- [x] [T035] [P0] encrypt 属性真实化：`macros/src/codegen/load.rs` 对 encrypt 字段在合并后反序列化前生成解密步骤（值匹配 envelope 才解密，明文直通；KeyProvider 由生成函数参数注入，缺省读 `CONFERS_MASTER_KEY` 的 EnvKeyProvider）；`encrypt = "aes256-gcm"` 生成 compile_error；`tests/e2e/encryption_e2e.rs` 改为断言解密后明文；新增明文/密文双路径测试 → macros/src/codegen/load.rs
- [x] [T036] [P0] 无特性 enc 值告警：`src/loader.rs` 与校验器对 `enc:` 前缀值在 encryption 特性未启用时产生 tracing warning + 校验 warning 项；`src/cli/mod.rs:1830-1838` doctor 无特性分支返回 warning 检查项；新增两处测试 → src/cli/mod.rs
- [x] [T037] [P0] 弱密钥拒绝：`src/secret/crypto.rs` 与 `key_provider.rs` 增加全零/全 FF/单字节重复预检返回 `CryptoError::WeakKey`；doctor ASCII 口令通道要求 ≥16 字节；SECURITY.md:155 示例改随机密钥；`FileKeyProvider` 文件权限非 0600 告警；新增四项测试 → src/secret/crypto.rs
- [x] [T038] [P1] envelope 统一带密钥版本：`src/security/prefix.rs` 提供唯一 parse/serialize `enc:v1:<keyver>:<payload>`（旧格式兼容读默认 v1，写入带 keyver）；doctor 与 e2e 格式迁移；新增往返与兼容测试 → src/security/prefix.rs
- [x] [T039] [P1] Vault token 续期：`src/secret/providers.rs:349-409` 403 清 token_cache 重登一次，按 lease_duration 提前 10% 刷新；新增 mock 过期重登测试 → src/secret/providers.rs
- [x] [T040] [P2] rotate 验证旧钥：`src/key/storage.rs:660-672` `rotate_master_key` 先以旧钥对已知密文做 HMAC 挑战验证再轮换，失败返回错误；新增正反测试 → src/key/storage.rs
- [x] [T041] [P0] 明文密钥受管化：`src/secret/crypto.rs` `derive_field_key` 返回 `Zeroizing<[u8;32]>`、`key_registry.rs` `fetch_and_register` 返回 SecretBytes、`src/key/mod.rs` generate/get_plaintext_key 中间串 Zeroizing、`crypto.rs` decrypt 返回 `Zeroizing<Vec<u8>>`，调用点同步适配；新增 zeroize 断言测试 → src/secret/crypto.rs
- [x] [T042] [P2] SecureString 定长掩码：`src/security/secure_string.rs:291-347` `masked()` 改固定 8 星号（不泄露前缀与长度）；新增测试断言长短值掩码一致 → src/security/secure_string.rs
- [x] [T043] [P1] 敏感文件 0600：`src/key/storage.rs`、`src/impl_/snapshot.rs`、`src/impl_/audit.rs` 落盘统一 unix 0600（新文件创建即设权限）；新增权限断言测试 → src/key/storage.rs
- [x] [T044] [P2] 加密清理：Cargo.toml 移除 secrecy/aes-gcm 死依赖与 KeyCachePolicy 死类型；`src/types.rs:1089` ZeroizingBytes drop 清 full capacity；SECURITY.md/API_REFERENCE 的 `CONFERS_ENCRYPTION_KEY` 改 `CONFERS_MASTER_KEY`；全量编译+测试确认无引用残留 → Cargo.toml
- [x] [T045] [P0] 快照接入敏感路径：`src/impl_/config/builder.rs:375` `save_blocking(merged, &[])` 改传 `T::sensitive_paths()`；新增 e2e 断言 sensitive 字段在快照 JSON 中为 `[REDACTED]` 且文件权限 0600 → src/impl_/config/builder.rs
- [x] [T046] [P1] CLI inspect/get 默认脱敏：`src/cli/mod.rs` inspect（text/json）与 get 默认按字段名规则掩码，新增 `--reveal` 显式明文并印 stderr 警告；新增 CLI 测试断言 password 字段默认掩码、--reveal 显示原文 → src/cli/mod.rs
- [x] [T047] [P1] 脱敏规则收敛：`src/error/sanitize.rs` 的 sanitize 函数并入 `security::ErrorSanitizer` 字段名驱动规则（password=xxx 值掩码），`src/error.rs` `user_message()` 真实调用 sanitize（修 InvalidValue URL 直通），i18n 模板 `locales/en/errors.ftl:11` 移除明文 message 插值；新增 password=xxx 掩码与 URL 脱敏测试 → src/error/sanitize.rs
- [x] [T048] [P1] 冲突报告值脱敏：`src/impl_/merger/engine.rs:131-135` 与 `src/types.rs:931-946` conflict_report 的 low/high value 按 sensitive_paths 掩码后再入报告；新增测试断言敏感冲突值在报告中为掩码 → src/impl_/merger/engine.rs
- [x] [T049] [P1] 敏感词表统一：`src/security/patterns.rs` 增补 authorization/passwd/pwd/dsn/bearer 并放宽复数形边界（命中点后允许 s），`src/impl_/audit.rs:475-500` 关键词判定改调用 patterns 单一来源；新增 authorization/passwords 命中与 monkey 不命中测试 → src/security/patterns.rs
- [x] [T050] [P2] 审计与掩码收尾：`src/security/config_injector.rs:594-604` mask_value 改固定 8 星号；`src/impl_/audit.rs` AuditSink 收 sanitize 后事件、verify_audit_chain 改常量时间比较、AuditConfig 新增 with_hmac_key 外置密钥（缺省沿用 salt）；新增四项测试 → src/impl_/audit.rs
- [x] [T051] [P1] 文档对齐与 CHANGELOG：README/ARCHITECTURE/API_REFERENCE/USER_GUIDE/SECURITY 按最终实现修正优先级描述、env 嵌套示例、渐进发布措辞、熔断覆盖面、总线可靠性等级、审计威胁模型与密钥 env 名；CHANGELOG 记录全部行为变更清单；typos/pre-commit 全量通过 → CHANGELOG.md

## Phase 1: Convergence

_由 /specmark converge 于 2026-09-23 生成。仅追加：不要编辑之前的任务。_

**发现缺口：** 2 (CRITICAL: 0 | HIGH: 0 | MEDIUM: 2 | LOW: 0)
**追加任务：** 2（跳过：0 个 LOW/unrequested）
**未请求范围（按原样接受）：**
- D2 精化：无前缀 env 源的环境固有冲突（如 CARGO vs CARGO_HOME）以「嵌套形状胜出 + telemetry 事件」确定性解决，仅前缀源/代码键硬报错（design.md 已回填）
- T044 精化：KeyCachePolicy 由"删除"改为"真实接线"——Vault token 缓存现按 NoCache/CacheWithTtl/CacheIndefinitely 生效（优于删除死类型）
- T028：`snapshot restore` 子命令按 `snapshot` 特性门控（cli 单特性构建不引入 snapshot 依赖）

**验收标准检查（5 个 delta spec，32 条 Requirement 全部 ✓ PASS）：**

| R-ID | 验收条件（摘要） | 状态 |
|------|------------------|------|
| R-loading-001..009 | 声明顺序优先级/env 冲突确定性/宏默认值/null 一致/错误带路径/separator/策略+profile/插值+告警/引擎一致 | ✓ PASS（chain 90、core 117、macro_e2e 20、engine 51 测试） |
| R-watch-001..009 | 原子替换存活/丢事件计数/panic 隔离/handle 单例/并发有序/Lagged/去抖接线/无 TTL 回退/渐进接线 | ✓ PASS（watcher 49、watch_e2e 6、concurrency 7、dynamic 15、progressive 13） |
| R-remote-001..010 | SSRF 补全/Consul 删除/K8s CA/超时+header/容灾选项/Redis 重连/epoch/Nacos 认证/熔断全覆盖+毒消息/快照原子 | ✓ PASS（remote 219、bus 54；NATS 服务用例为环境门控，HEAD 实证同等失败） |
| R-crypto-001..008 | encrypt 真实解密/无特性告警/弱密钥拒绝/envelope 统一/Vault 续期/rotate 验旧钥/受管类型/清理 | ✓ PASS（encryption_e2e 2、key_e2e 5、security 39、secret 127、t035-t042/t044 专项） |
| R-mask-001..008 | 快照脱敏 0600/CLI 默认脱敏/规则收敛/冲突报告脱敏/词表统一/定长掩码/审计加固/文档对齐 | ✓ PASS（cli 147、audit_e2e 16、t045-t051 专项） |

**收敛验证矩阵（全部通过）：**
- `cargo test --lib --features full`：2211 passed / 8 failed（均为 bus::nats 服务门控，干净 HEAD worktree 实证同等失败，非回归）
- 16 个集成测试目标（full 特性）：538 passed / 0 failed
- `cargo test --lib --no-default-features`：673 passed / 0 failed
- `cargo clippy --workspace --all-targets --features full -- -D warnings`：干净
- `cargo fmt --check`：零差异；`cargo test -p confers-macros --test compile_fail`：通过（含新增 aes256-gcm 拒绝用例）

- [x] [T052] [P1] aes256-gcm 编译期拒绝固化于 trybuild（macros/tests/compile_fail/t035_*.rs + stderr） → macros/tests/compile_fail/t035_aes256_gcm_not_implemented.rs
- [x] [T053] [P1] 弱密钥检测单测（is_weak_key 表驱动 + EnvKeyProvider::build 拒绝路径） → src/secret/crypto.rs
