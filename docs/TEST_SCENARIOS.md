# confers 验收测试场景穷举矩阵

> 适用版本：confers **0.6.0-rc.2**（workspace，Rust 1.97.1 / edition 2024）
> 用途：7 仓库统一 E2E 验收工程第一步 —— 先穷举全部验收场景，后续按本文档逐条固化为 `tests/e2e/` 下的 E2E 测试。
> 编写依据（只读核对）：`Cargo.toml [features]`、`src/lib.rs` 导出面、`src/cli/mod.rs` 子命令、`macros/src/parse.rs` 属性表、`tests/{core,security,remote,watcher,cli}/`、src 内 61 个 `#[cfg(test)]` 模块、`examples/` 21 个示例、`docker-compose.test.yml`。
> 所有引用的既有测试名均经 `grep` 核实存在。

## 阅读约定

- **类型**：`正常`（合法输入下的预期行为）/ `异常`（错误输入、故障注入，须给出明确错误）/ `边界`（极限值、并发、竞态、组合临界）。
- **既有覆盖**：`文件::测试名` 表示已有测试（含 `src/**` 内联测试模块）；标注 `⚠死文件` 表示文件存在但**未在对应 `mod.rs` 注册、实际不执行**（见下文重要发现）；`无→需新增` 表示无既有覆盖。
- **E2E 落点**：计划写入 `tests/e2e/` 的目标文件（后续落地阶段创建）。
- 依赖服务取值：`无` / `本地文件` / `NATS(4222)` / `Redis(16379)` / `etcd(2379)` / `Consul(8500)`（端口见 §4）。

### 重要发现（编写时盘点得出）

1. **`tests/core/error.rs`（42 个测试）与 `tests/core/encryption.rs`（32 个测试）是死文件**：`tests/core/mod.rs` 只注册了 `context/coverage/derive/dynamic/env_types/load/merge/migration/modules/nested_deserialize/progressive/toggle` 12 个子模块，`error` 与 `encryption` 未注册，**当前 cargo test 不执行它们**。其中 encryption 与 `tests/security/encryption.rs`（已注册）内容重复；error 测试则**仅存在于死文件中**。落地 E2E 前应先把 error 测试迁移注册（或在 e2e 重写），引用处均标注 `⚠死文件`。
2. `tests/remote/remote.rs` 内含与 `tests/remote/etcd.rs`、`tests/remote/consul.rs` 重复的 source 测试（两处均编译执行）。
3. `Cargo.toml` 声明 `autotests = false`，新增 E2E 集成测试必须**显式添加 `[[test]]` 段**并按需声明 `required-features`。
4. 空文件 `tests/core/load.rs::test_load_empty_file` 等已确认：空文件解析为空 Map 而非错误。

---

## 1. 总览：功能域 × feature 分组

| # | 功能域 | 场景 ID 前缀 | 涉及 feature（Cargo.toml 名） | 主要公共 API |
|---|--------|-------------|------------------------------|--------------|
| 1 | 格式解析与 Loader | FMT | toml, json, yaml, ini, env, dotenv（default=toml+json+env） | `loader::{Format, LoaderConfig, load_file, parse_content, detect_format_from_path, detect_format_from_content, parse_toml/json/yaml/ini}` |
| 2 | 配置构建 / Source 链 / 合并 | BLD | default + remote(AsyncSource)、snapshot(with_snapshot) | `config::{config, ConfigBuilder, ConfigLimits, SourceChainBuilder, DefaultSource, EnvSource, FileSource, MemorySource, Source, SourceKind}`、`merger::MergeStrategy`、`interface::{ConfigConnector/Reader/Writer/Provider/Ext}`、`new_in_memory()` |
| 3 | 校验 | VAL | validation | `validator::{Validate, ValidationResult, ValidationRule}` |
| 4 | 插值 | IPL | interpolation | `interpolation::{interpolate, interpolate_tracked, InterpolationConfig, InterpolationContext, InterpolationResult, InterpolationWarning}` |
| 5 | 加密与 Secret | ENC | encryption（security→encryption） | `secret::{SecretString, SecretBytes, XChaCha20Crypto, CryptoError, derive_field_key, EnvKeyProvider(+Builder), SecretKeyProvider, FileKeyProvider(+Builder), VaultKeyProvider(+Builder), KeyRegistry(+Builder), ZeroizingBytes}` |
| 6 | 密钥管理 | KEY | key（→encryption） | `key::{KeyManager, KeyRing, KeyBundle, KeyMetadata, KeyRotationSchedule, RotationPlan, KeyStatus}` |
| 7 | 安全规则与注入 | SEC | security, security-rules（→security→encryption） | `security::{EnvSecurityValidator, EnvironmentValidationConfig, config_injector, error_sanitization}`、`security::rules::{SecurityValidatorRegistry, CorsValidator, SsrfValidator, JwtSecretValidator, TlsConfigValidator, SecurityReport, SecurityViolation, ViolationSeverity}` |
| 8 | 审计 | AUD | audit | `audit::{AuditEvent, AuditLevel, AuditConfig(+Builder), AuditWriter(+Builder)}` |
| 9 | 文件热更新 | WAT | watch | `watcher::{FsWatcher, MultiFsWatcher, WatcherConfig(+Builder), WatcherGuard, AdaptiveDebouncer}` |
| 10 | 渐进重载 | PGR | progressive-reload（→watch） | `watcher::{ProgressiveReloader(+Builder), ReloadHealthCheck, HealthStatus, ReloadOutcome}` |
| 11 | 动态字段 | DYN | dynamic（+watch 的 FieldWatcher） | `dynamic::{DynamicField(+Builder), CallbackGuard, FieldWatcher}` |
| 12 | 特性开关 | TGL | feature-toggle | `toggle::{FeatureToggleRegistry, FeatureInfo}` |
| 13 | 版本迁移 | MIG | migration | `migration::{Versioned, MigrationRegistry, MigrationFn, MigrationOnReload}`、派生宏 `ConfigMigration` |
| 14 | 快照 | SNP | snapshot（→dynamic,json,toml,yaml） | `snapshot::{SnapshotManager, SnapshotConfig, SnapshotFormat, SnapshotInfo}`、`ConfigBuilder::with_snapshot` |
| 15 | 模块 / 配置分组 | MOD | modules（→toml） | `modules::{ModuleConfig, ModuleRegistry}`、派生宏 `ConfigModules` |
| 16 | 上下文感知 | CTX | context-aware | `context::{ContextAwareField(+Builder), ContextRule, ContextValue, EvaluationContext}` |
| 17 | HTTP 轮询远程源 | REM | remote | `remote::{HttpPolledSource(+Builder), PolledSource}`、`config::AsyncSource` |
| 18 | etcd | ETC | etcd（→remote） | etcd source builder（endpoint/auth/prefix/interval/tls） |
| 19 | Consul | CSL | consul（→remote） | consul source builder（address/token/prefix/interval/tls、max_response_bytes/max_kv_entries） |
| 20 | 配置总线（进程内） | BUS | config-bus | `bus::{ConfigBus, InMemoryBus, BusBuilder, ConfigChangeEvent, BusEventLimiter}` |
| 21 | NATS 总线 | NAT | nats-bus（→config-bus） | `bus::NatsConfigBus, NatsBusBuilder` |
| 22 | Redis 总线 | RDS | redis-bus（→config-bus） | `bus::RedisConfigBus, RedisBusBuilder` |
| 23 | Schema | SCH | schema, typescript-schema | 派生宏 `ConfigSchema`、`schema::TypeScriptGenerator` |
| 24 | CLI | CLI | cli（→toml,json,yaml） | `cli::run::<T>`、二进制 `confers`（Inspect/Validate/Export/Diff/Snapshot） |
| 25 | 派生宏与属性 | MAC | —（配合各 feature） | `Config`、`ConfigClap`、`ConfigMigration`、`ConfigModules`、`ConfigSchema`；结构属性 `validate/env_prefix/app_name/strict/watch/version/profile/profile_env`；字段属性 `default/description/name/name_env/name_clap_long/name_clap_short/sensitive/encrypt/flatten/skip/interpolate/merge_strategy/dynamic/module_group` |
| 26 | feature 组合交互 | CMP | 多 feature 叠加 | — |
| 27 | 并发与竞态 | CCY | dynamic, watch, feature-toggle, config-bus, snapshot, audit | — |
| 28 | feature 预设编译矩阵 | PRS | recommended/dev/production/full/minimal/distributed + 单 feature | `cargo check --features …` |

场景总数：**357**（正常 152 + 双断言 8+3 / 异常 84 / 边界 110，程序化核对见 §6）。

---

## 2. 场景矩阵

### 2.1 格式解析与 Loader（FMT，22 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| FMT-01 | 合法 TOML 文件经 `load_file` 解析为 Map，键值与文件一致 | 正常 | toml | 本地文件 | `tests/core/load.rs::test_load_with_format_detection` | tests/e2e/format_e2e.rs |
| FMT-02 | 合法 JSON 文件解析，嵌套对象/数组保留结构 | 正常 | json | 本地文件 | `tests/core/load.rs::test_load_array_values`、`tests/core/nested_deserialize.rs::test_nested_json_deserializes_into_struct` | tests/e2e/format_e2e.rs |
| FMT-03 | 合法 YAML 文件解析（含嵌套 map） | 正常 | yaml | 本地文件 | `tests/core/nested_deserialize.rs::test_nested_yaml_deserializes_into_struct` | tests/e2e/format_e2e.rs |
| FMT-04 | 合法 INI 文件解析为扁平键值（ini 无嵌套语义，验证 [section] 前缀化） | 正常 | ini | 本地文件 | src 内联 `src/impl_/loader.rs` tests | tests/e2e/format_e2e.rs |
| FMT-05 | `detect_format_from_path`：.toml/.json/.yaml/.yml/.ini 各扩展名返回对应 Format | 正常 | toml,json,yaml,ini | 无 | `tests/core/coverage.rs::test_detect_from_path_cases` | tests/e2e/format_e2e.rs |
| FMT-06 | `detect_format_from_content`：无扩展名文件按内容（`{`/`key =`/`key:`）嗅探 | 边界 | toml,json,yaml | 无 | `tests/core/coverage.rs::test_detect_from_content_formats` | tests/e2e/format_e2e.rs |
| FMT-07 | TOML 语法错误（如 `key =`）→ `ConfigError`，`ConfigErrorCode=2300(CONFIG_PARSE_ERROR)`，携带行列 `ParseLocation` | 异常 | toml | 本地文件 | `tests/core/load.rs::test_load_invalid_toml`；`⚠死文件 tests/core/error.rs::test_invalid_toml_format_error/test_parse_error_location` | tests/e2e/format_e2e.rs |
| FMT-08 | JSON 语法错误（截断 JSON）→ 解析错误含位置 | 异常 | json | 本地文件 | `tests/core/load.rs::test_load_invalid_json`；`⚠死文件 tests/core/error.rs::test_invalid_json_format_error` | tests/e2e/format_e2e.rs |
| FMT-09 | YAML 语法错误（tab 缩进等）→ 解析错误 | 异常 | yaml | 本地文件 | `⚠死文件 tests/core/error.rs::test_invalid_yaml_format_error` | tests/e2e/format_e2e.rs |
| FMT-10 | 空文件（0 字节）→ 成功返回空 Map，不报错 | 边界 | toml | 本地文件 | `tests/core/load.rs::test_load_empty_file` | tests/e2e/format_e2e.rs |
| FMT-11 | 文件超过 `LoaderConfig::max_size` → 大小超限错误（2400 族），不读入内存 | 异常 | toml | 本地文件 | `⚠死文件 tests/core/error.rs::test_size_limit_exceeded_error`（单测层面）；无 loader 级集成→需新增 | tests/e2e/format_e2e.rs |
| FMT-12 | `load_file` 不存在的路径 → `ConfigFileNotFound(2200)`，错误含 source 信息 | 异常 | default | 本地文件 | `tests/core/load.rs::test_load_file_not_found`；`⚠死文件 tests/core/error.rs::test_file_not_found_with_source` | tests/e2e/format_e2e.rs |
| FMT-13 | 路径穿越：`config/../../etc/passwd.toml` 被 `normalize_and_validate_path` 拒绝 | 异常 | default | 本地文件 | src 内联 `src/impl_/loader.rs`（`check_path_traversal_attempt` tests） | tests/e2e/format_e2e.rs |
| FMT-14 | 符号链接指向 allowed_dir 之外 → 默认拒绝；`no_symlink_check()` 放行 | 异常 | default | 本地文件 | src 内联 `src/impl_/loader.rs` | tests/e2e/format_e2e.rs |
| FMT-15 | 绝对路径默认拒绝；`allow_absolute()` / `ConfigBuilder::allow_absolute_paths()` 放行 | 异常 | default | 本地文件 | `tests/cli/commands.rs::test_diff_command`（绝对路径 bail 分支）；src 内联 loader tests | tests/e2e/format_e2e.rs |
| FMT-16 | 未知扩展名且内容无法嗅探 → `Format::try_parse`/detect 返回 None，调用方报"未知格式" | 异常 | default | 本地文件 | `tests/core/coverage.rs::test_format_try_parse_all`（反向断言） | tests/e2e/format_e2e.rs |
| FMT-17 | `.env` 文件经 dotenvy 加载为 env 源；`dotenv` 为 `env` 的 alias | 正常 | env,dotenv | 本地文件 | `tests/cli/commands.rs::test_env_file_loading`、`tests/core/derive.rs::test_env_file_loading` 所在族 | tests/e2e/format_e2e.rs |
| FMT-18 | INI 只支持一层 section：深层嵌套输入不丢失也不 panic（降级为字符串） | 边界 | ini | 本地文件 | src 内联 loader tests（parse_ini 分支）→部分，需新增断言 | tests/e2e/format_e2e.rs |
| FMT-19 | 带 UTF-8 BOM 的 TOML/JSON 文件可解析（或按实现明确报错，二选一固化） | 边界 | toml,json | 本地文件 | 无→需新增 | tests/e2e/format_e2e.rs |
| FMT-20 | 同一内容 `load_file(path)` 与 `parse_content(content, detect(path))` 结果等价（值+source） | 正常 | toml,json | 本地文件 | `tests/core/load.rs::test_load_with_format_detection`（部分）→需新增等价断言 | tests/e2e/format_e2e.rs |
| FMT-21 | 三种格式同一配置语义：TOML/JSON/YAML 各写一份相同嵌套结构，解析后 Map 深度相等 | 正常 | toml,json,yaml | 本地文件 | `tests/core/nested_deserialize.rs`（3 例） | tests/e2e/format_e2e.rs |
| FMT-22 | 未启用 toml/json feature 时 `parse_toml/parse_json` 返回"未编译该格式"占位错误（cfg 分支） | 异常 | （feature 关闭态） | 无 | src 内联 `src/impl_/loader.rs` stub 分支 | tests/e2e/format_e2e.rs（no-default 特性编译） |

### 2.2 配置构建 / Source 链 / 合并（BLD，28 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| BLD-01 | `config::<T>().file(f).build()` 反序列化为 typed struct，字段全命中 | 正常 | default | 本地文件 | `tests/core/derive.rs::test_simple_config_load`、`tests/core/load.rs::test_config_builder_creation` | tests/e2e/builder_e2e.rs |
| BLD-02 | `build_annotated()` 返回 `AnnotatedValue`：每个叶子带 source 与 location | 正常 | default | 本地文件 | `tests/cli/commands.rs::test_export_with_provenance`（CLI 面）；库面→需新增直接断言 | tests/e2e/builder_e2e.rs |
| BLD-03 | 单 `FileSource`（`.file()`）加载成功，SourceKind=file | 正常 | default | 本地文件 | `tests/core/coverage.rs::test_file_source_basic` | tests/e2e/builder_e2e.rs |
| BLD-04 | 两个 `.file()`：后加入的高优先级覆盖前者同名键 | 正常 | default | 本地文件 | `tests/core/merge.rs::test_precedence_file_shadows_default` 族 | tests/e2e/builder_e2e.rs |
| BLD-05 | `.env()` 收集 `APP_xxx` 类环境变量并按 `__`/嵌套规则建树 | 正常 | env | 无 | `tests/core/env_types.rs::test_env_vars_deserialize_into_typed_struct`、`test_env_deep_nesting_builds_nested_tree` | tests/e2e/builder_e2e.rs |
| BLD-06 | `.env_prefix("MYAPP_")` 只收集带前缀变量并剥前缀 | 正常 | env | 无 | `tests/core/derive.rs::test_prefixed_config_env_mapping` | tests/e2e/builder_e2e.rs |
| BLD-07 | `.defaults(map)` 提供全量默认值 | 正常 | default | 无 | `tests/core/merge.rs::test_precedence_default_file_env_memory` | tests/e2e/builder_e2e.rs |
| BLD-08 | `.default(key, value)` 单键默认，可被文件/env 覆盖 | 正常 | default | 无 | `tests/core/derive.rs::test_simple_config_default` | tests/e2e/builder_e2e.rs |
| BLD-09 | `.memory(map)` 内存源注入 | 正常 | default | 无 | `tests/core/coverage.rs::test_memory_source_basic/with_initial_values` | tests/e2e/builder_e2e.rs |
| BLD-10 | `.memory_priority(n)` 调整内存源优先级 | 边界 | default | 无 | `tests/core/coverage.rs::test_memory_source_not_optional` 附近族；专用断言→需新增 | tests/e2e/builder_e2e.rs |
| BLD-11 | 完整优先级链 default < file < env < memory：同一键四处定义，最终取 memory | 正常 | env | 无 | `tests/core/merge.rs::test_precedence_default_file_env_memory` | tests/e2e/builder_e2e.rs |
| BLD-12 | 全局 `MergeStrategy` 六种：Replace/Join/Append/Prepend/JoinAppend/DeepMerge 各验证合并结果 | 正常 | default | 无 | `tests/core/merge.rs::test_merge_replace_strategy` 等 6 例 | tests/e2e/builder_e2e.rs |
| BLD-13 | `.field_strategy("tags", Append)` 字段级策略覆盖全局策略 | 边界 | default | 无 | `tests/core/merge.rs::test_field_specific_strategy` | tests/e2e/builder_e2e.rs |
| BLD-14 | `ConfigLimits::max_file_size_bytes` 超限 → `ConfigSizeLimitExceeded(2400)` | 异常 | default | 本地文件 | `⚠死文件 tests/core/error.rs::test_size_limit_exceeded_error`；集成级→需新增 | tests/e2e/builder_e2e.rs |
| BLD-15 | 嵌套深度超 `max_nesting_depth` → 明确错误 | 异常 | default | 本地文件 | src 内联 limits tests（`src/impl_/config/limits.rs`）→需集成新增 | tests/e2e/builder_e2e.rs |
| BLD-16 | 键总数超 `max_total_fields` → 超限错误 | 异常 | default | 本地文件 | src 内联 limits tests→需集成新增 | tests/e2e/builder_e2e.rs |
| BLD-17 | 数组长度超 `max_array_length` → 超限错误 | 异常 | default | 本地文件 | src 内联 limits tests→需集成新增 | tests/e2e/builder_e2e.rs |
| BLD-18 | 字符串超 `max_string_length` → 超限错误 | 异常 | default | 本地文件 | src 内联 limits tests→需集成新增 | tests/e2e/builder_e2e.rs |
| BLD-19 | 必填字段缺失（无 default 的非 Option 字段）→ `MissingField(2001)` | 异常 | default | 本地文件 | `⚠死文件 tests/core/error.rs::test_missing_required_field_error/test_missing_nested_required_field` | tests/e2e/builder_e2e.rs |
| BLD-20 | 类型不匹配（port="abc" 对 u16）→ 类型错误而非 panic；int↔string 互转边界各自报错 | 异常 | default | 本地文件 | `⚠死文件 tests/core/error.rs::test_type_mismatch_error/test_integer_to_string_mismatch/test_string_to_integer_mismatch/test_object_to_primitive_mismatch` | tests/e2e/builder_e2e.rs |
| BLD-21 | `build_resilient()` + `fail_fast(false)`：单源损坏不中断，`BuildResult::Degraded` 携带 `SourceWarning` | 边界 | default | 本地文件 | `⚠死文件 tests/core/error.rs::test_build_result_degraded/test_build_result_with_warnings/test_multi_source_error_partial_config` | tests/e2e/builder_e2e.rs |
| BLD-22 | `build_with_fallback(fallback)`：所有源失败时回退默认实例并标记 warning | 异常 | default | 本地文件 | src 内联 builder tests（`src/impl_/config/builder.rs`）→需集成新增 | tests/e2e/builder_e2e.rs |
| BLD-23 | `.file_optional(不存在路径)` → 跳过该源，构建成功 | 边界 | default | 本地文件 | `tests/core/coverage.rs::test_file_source_optional_flag` | tests/e2e/builder_e2e.rs |
| BLD-24 | `.file(不存在路径)`（非 optional）→ `ConfigFileNotFound(2200)` | 异常 | default | 本地文件 | `tests/core/load.rs::test_load_file_not_found` | tests/e2e/builder_e2e.rs |
| BLD-25 | `SourceChainBuilder` 手工组装 DefaultSource/EnvSource/FileSource/MemorySource 自定义链 | 正常 | env | 无 | `tests/core/coverage.rs::test_source_kind_values`、src 内联 chain tests（`src/impl_/config/chain.rs`） | tests/e2e/builder_e2e.rs |
| BLD-26 | 合并遇到 Null 值：null 覆盖/被覆盖语义固化（Replace 下 null 覆盖标量） | 边界 | default | 无 | `tests/core/merge.rs::test_merge_null_values` | tests/e2e/builder_e2e.rs |
| BLD-27 | 空配置：无任何源 + 全 Option 字段 struct → 构建成功为全默认；全必填 → MissingField | 边界 | default | 无 | `tests/core/coverage.rs::test_default_source_collect_empty`、`tests/cli/commands.rs::test_build_annotated_from_cli_empty_paths` 同族 | tests/e2e/builder_e2e.rs |
| BLD-28 | `new_in_memory()` 工厂：set/get_string/has/delete/clear/health_check/shutdown 全链路 | 正常 | default | 无 | src 内联 `src/impl_/memory.rs` tests、`tests/core/dynamic.rs::test_real_config_provider` | tests/e2e/builder_e2e.rs |

### 2.3 校验（VAL，8 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| VAL-01 | `#[derive(Config)] #[config(validate = true)]` 合法配置通过校验并成功 build | 正常 | validation | 本地文件 | `tests/core/coverage.rs::test_validate_trait_is_accessible`；端到端→需新增 | tests/e2e/validation_e2e.rs |
| VAL-02 | 非法值（如 email 格式错 / range 越界）→ `ConfigValidationFailed(2500)`，错误含字段路径 | 异常 | validation | 本地文件 | `tests/core/error.rs`⚠死文件`::test_config_validation_failure`（已注册版在 src 内联 validator tests） | tests/e2e/validation_e2e.rs |
| VAL-03 | `ValidationRule::from_str("length(1..)")/"range(0..100)"` 解析为规则 | 正常 | validation | 无 | `tests/core/coverage.rs::test_validation_rule_from_str_length/range/simple` | tests/e2e/validation_e2e.rs |
| VAL-04 | 非法规则字符串 → 解析错误（而非静默忽略） | 异常 | validation | 无 | `tests/core/coverage.rs::test_validation_rule_from_str_invalid` | tests/e2e/validation_e2e.rs |
| VAL-05 | 自定义 validator 注册进链路并被调用 | 正常 | validation | 无 | src 内联 `src/impl_/validator.rs` tests | tests/e2e/validation_e2e.rs |
| VAL-06 | garde 报告 → `ConfigError` 转换保留字段信息；`user_message` 输出可读文案 | 异常 | validation | 无 | `⚠死文件 tests/core/error.rs::test_validation_error_from_garde_report/test_validation_error_user_message` | tests/e2e/validation_e2e.rs |
| VAL-07 | 多字段同时违规 → 错误聚合为一条含全部字段的结果 | 边界 | validation | 无 | src 内联 validator tests→需集成新增 | tests/e2e/validation_e2e.rs |
| VAL-08 | ValidationRule 相等性/Clone/Debug（规则作为配置的一部分可比较） | 边界 | validation | 无 | `tests/core/coverage.rs::test_validation_rule_equality/test_validation_rule_clone_and_debug/test_validation_result_type_alias` | tests/e2e/validation_e2e.rs |

### 2.4 插值（IPL，9 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| IPL-01 | `${VAR}` 由 resolver 解析替换（`interpolate("Server: ${HOST}")` → `Server: localhost`） | 正常 | interpolation | 无 | src 内联 `src/impl_/interpolation.rs::`（line 649 断言） | tests/e2e/interpolation_e2e.rs |
| IPL-02 | `${VAR:8080}` 默认值语法：VAR 缺失时取默认 | 正常 | interpolation | 无 | src 内联 interpolation tests（doc 与 tests 覆盖） | tests/e2e/interpolation_e2e.rs |
| IPL-03 | 未定义变量且无默认 → `InterpolationConfigError(2800)`/undefined variable 错误 | 异常 | interpolation | 无 | `⚠死文件 tests/core/error.rs::test_undefined_variable_error` | tests/e2e/interpolation_e2e.rs |
| IPL-04 | 循环引用 A→B→A → `ConfigCircularReference(2900)` | 异常 | interpolation | 无 | `⚠死文件 tests/core/error.rs::test_circular_reference_detection` | tests/e2e/interpolation_e2e.rs |
| IPL-05 | 自引用 `${A}` 定义于 A 自身 → 报循环 | 异常 | interpolation | 无 | `⚠死文件 tests/core/error.rs::test_self_reference_error` | tests/e2e/interpolation_e2e.rs |
| IPL-06 | 嵌套循环链（3+ 节点）检测并报路径 | 异常 | interpolation | 无 | `⚠死文件 tests/core/error.rs::test_nested_circular_reference` | tests/e2e/interpolation_e2e.rs |
| IPL-07 | 敏感变量标记：`interpolate_tracked("${API_KEY}", …, true)` → `InterpolationResult::has_sensitive_refs/referenced("API_KEY")`；`InterpolationContext.record` 后 `is_sensitive_ref` 生效 | 正常 | interpolation | 无 | src 内联 interpolation tests（sensitive 断言组） | tests/e2e/interpolation_e2e.rs |
| IPL-08 | 截断模板 `"${VAR"`（未闭合）→ 明确解析错误不 panic | 异常 | interpolation | 无 | src 内联 interpolation tests（line ~798 断言 Err） | tests/e2e/interpolation_e2e.rs |
| IPL-09 | `examples/interpolation` 全链路：文件+env 混合模板插值输出（示例可跑通） | 边界 | interpolation,env | 本地文件 | examples/interpolation（运行验收） | tests/e2e/combo_e2e.rs（引用示例） |

### 2.5 加密与 Secret（ENC，23 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| ENC-01 | `XChaCha20Crypto` encrypt→decrypt roundtrip 内容一致 | 正常 | encryption | 无 | `tests/security/encryption.rs::test_encrypt_decrypt_roundtrip` | tests/e2e/encryption_e2e.rs |
| ENC-02 | 同一明文两次加密 nonce 唯一（密文不同） | 边界 | encryption | 无 | `tests/security/encryption.rs::test_encrypt_produces_unique_nonces` | tests/e2e/encryption_e2e.rs |
| ENC-03 | 用错误密钥解密 → `CryptoError`，不返回明文 | 异常 | encryption | 无 | `tests/security/encryption.rs::test_decrypt_with_wrong_key_fails` | tests/e2e/encryption_e2e.rs |
| ENC-04 | 篡改 nonce 解密 → 失败 | 异常 | encryption | 无 | `tests/security/encryption.rs::test_decrypt_with_wrong_nonce_fails` | tests/e2e/encryption_e2e.rs |
| ENC-05 | 密钥长度非 32 字节（短/长）→ 构造/加密报 `InvalidKeyLength` | 异常 | encryption | 无 | `tests/security/encryption.rs::test_invalid_key_length_fails/test_encrypt_key_too_short/test_encrypt_key_too_long` | tests/e2e/encryption_e2e.rs |
| ENC-06 | 空明文加解密 roundtrip | 边界 | encryption | 无 | `tests/security/encryption.rs::test_encrypt_empty_data` | tests/e2e/encryption_e2e.rs |
| ENC-07 | 大明文（MB 级）加解密 roundtrip | 边界 | encryption | 无 | `tests/security/encryption.rs::test_encrypt_large_data` | tests/e2e/encryption_e2e.rs |
| ENC-08 | `derive_field_key`：同 master 同字段确定性；不同字段派生不同密钥 | 正常 | encryption | 无 | `tests/security/encryption.rs::test_derive_field_key/test_derive_field_key_deterministic/test_derive_field_key_different_fields` | tests/e2e/encryption_e2e.rs |
| ENC-09 | `SecretString` Debug 输出脱敏（不泄漏明文） | 正常 | encryption | 无 | `tests/security/encryption.rs::test_secret_string_debug_redacts` | tests/e2e/encryption_e2e.rs |
| ENC-10 | `SecretBytes` drop 后内存 zeroize（test 钩子验证缓冲区清零） | 边界 | encryption | 无 | `tests/security/encryption.rs::test_secret_bytes_drop_zeroizes` | tests/e2e/encryption_e2e.rs |
| ENC-11 | SecretString/SecretBytes clone/len/deref/default 语义 | 正常 | encryption | 无 | `tests/security/encryption.rs::test_secret_string_*`、`test_secret_bytes_clone/len` | tests/e2e/encryption_e2e.rs |
| ENC-12 | `EnvKeyProvider`：从环境变量读取 32 字节密钥成功 | 正常 | encryption | 无 | `tests/security/encryption.rs::test_env_key_provider/test_env_key_provider_exact_length` | tests/e2e/encryption_e2e.rs |
| ENC-13 | `EnvKeyProvider`：环境变量缺失 → 明确错误 | 异常 | encryption | 无 | `tests/security/encryption.rs::test_env_key_provider_missing_var/test_env_key_provider_builder_missing_env_var/test_env_key_provider_builder_no_env_var_set` | tests/e2e/encryption_e2e.rs |
| ENC-14 | 密钥过短/过长 → 拒绝并提示期望长度 | 异常 | encryption | 无 | `tests/security/encryption.rs::test_env_key_provider_too_short_key/test_env_key_provider_too_long_key` | tests/e2e/encryption_e2e.rs |
| ENC-15 | `EnvKeyProviderBuilder` 链式构建（含 var 名自定义） | 正常 | encryption | 无 | `tests/security/encryption.rs::test_env_key_provider_builder` | tests/e2e/encryption_e2e.rs |
| ENC-16 | `FileKeyProvider`：从文件读密钥；文件缺失/权限错误 → 报错 | 正常/异常 | encryption | 本地文件 | src 内联 `src/secret/providers.rs` tests | tests/e2e/encryption_e2e.rs |
| ENC-17 | `VaultKeyProvider`（encryption+remote 组合）：不可达 Vault 地址 → 连接错误而非 panic | 异常 | encryption,remote | 无（指向不可达地址） | src 内联 `src/secret/providers.rs` cfg(remote) tests | tests/e2e/encryption_e2e.rs |
| ENC-18 | `KeyRegistry`：rotate 后 `try_all_keys` 可用旧密钥解密旧数据、新密钥加密新数据 | 正常 | encryption | 无 | `tests/security/security.rs::test_key_registry_rotation/test_key_registry_try_all_keys` | tests/e2e/encryption_e2e.rs |
| ENC-19 | 完整工作流：provider→derive_field_key→encrypt→decrypt（`test_full_encryption_workflow` 同构端到端） | 正常 | encryption | 无 | `tests/security/encryption.rs::test_full_encryption_workflow/test_secret_string_with_encryption` | tests/e2e/encryption_e2e.rs |
| ENC-20 | 派生宏 `#[config(encrypt = "xchacha20")]` 字段：配置加载时密文解密注入，错误密钥 → 加载失败 | 正常/异常 | encryption,macros | 本地文件 | src 内联 `macros/src/codegen/security.rs` 相关；集成→需新增 | tests/e2e/encryption_e2e.rs |
| ENC-21 | `encrypt = "aes256-gcm"` 算法路径 roundtrip | 正常 | encryption | 无 | src 内联 crypto tests（aes-gcm 依赖）→需集成新增 | tests/e2e/encryption_e2e.rs |
| ENC-22 | 环境变量注入密文（`enc:<base64>` 形态）经 config_injector 解密为 SecretString | 正常 | security,encryption | 无 | src 内联 `src/security/config_injector.rs` tests | tests/e2e/encryption_e2e.rs |
| ENC-23 | 非法算法名 `encrypt = "rot13"` → 宏展开期编译错误（trybuild 级验收，提示支持列表） | 异常 | macros,encryption | 无 | macros crate 无 trybuild 用例→需新增 | tests/e2e/macro_e2e.rs（trybuild） |

### 2.6 密钥管理（KEY，13 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| KEY-01 | `KeyBundle::generate` 生成随机 32 字节密钥 + 元数据 | 正常 | key | 无 | src 内联 `src/key/mod.rs` tests | tests/e2e/key_e2e.rs |
| KEY-02 | `KeyBundle::get_plaintext_key(master_key)` 用正确 master 解出明文密钥；错误 master → `ConfigError` | 正常/异常 | key,encryption | 无 | src 内联 key tests | tests/e2e/key_e2e.rs |
| KEY-03 | `KeyRing::rotate` 产生新版本并切换默认版本 | 正常 | key | 无 | src 内联 `src/key/mod.rs` rotate tests | tests/e2e/key_e2e.rs |
| KEY-04 | `get_key_by_version(不存在版本)` → None；`deactivate_version` 后查不到 | 边界 | key | 无 | src 内联 key tests | tests/e2e/key_e2e.rs |
| KEY-05 | `add_secondary_key` 后多版本共存，旧版本仍可解密 | 正常 | key | 无 | src 内联 key tests | tests/e2e/key_e2e.rs |
| KEY-06 | `KeyRotationSchedule::is_rotation_due/days_until_rotation/update_after_rotation`（到期/未到期两分支） | 正常/边界 | key | 无 | src 内联 key tests | tests/e2e/key_e2e.rs |
| KEY-07 | `KeyManager::initialize/generate_key` 初始化与随机生成 | 正常 | key | 无 | src 内联 `src/key/manager.rs` tests | tests/e2e/key_e2e.rs |
| KEY-08 | `create_key_ring/rotate_key` 全流程；`get_rotation_status` 反映状态 | 正常 | key | 无 | src 内联 manager tests | tests/e2e/key_e2e.rs |
| KEY-09 | `get_key_info(不存在 key_id)` → 错误（`KeyRotationFailed` 族，见 875 行死文件测试名 `⚠死文件 tests/core/error.rs::test_key_rotation_failed_error`） | 异常 | key | 无 | src 内联 manager tests | tests/e2e/key_e2e.rs |
| KEY-10 | `plan_rotation/cleanup_old_keys(keep_n)`：清理后保留版本数正确、默认键不被清理 | 边界 | key | 无 | src 内联 manager tests | tests/e2e/key_e2e.rs |
| KEY-11 | `deprecate_version` 弃用版本后 get_key_by_version 仍可取但标记弃用 | 边界 | key | 无 | src 内联 manager tests | tests/e2e/key_e2e.rs |
| KEY-12 | `set_default_key_id(非法 id)` → 错误 | 异常 | key | 无 | src 内联 manager tests | tests/e2e/key_e2e.rs |
| KEY-13 | `KeyMetadata::is_expired/is_active` 时间边界（当天/过期瞬间） | 边界 | key | 无 | src 内联 `src/key/mod.rs` tests | tests/e2e/key_e2e.rs |

### 2.7 安全规则与注入（SEC，18 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| SEC-01 | `EnvSecurityValidator` 合法 env 名/值通过 `validate_env_name/validate_env_value` | 正常 | security | 无 | src 内联 `src/security/input_validation.rs` tests | tests/e2e/security_rules_e2e.rs |
| SEC-02 | env 名含注入字符（空格/`=`/换行/shell 元字符）→ `EnvSecurityError` | 异常 | security | 无 | src 内联 input_validation tests | tests/e2e/security_rules_e2e.rs |
| SEC-03 | env 名/值超过 `EnvironmentValidationConfig` 上限 → 拒绝 | 异常 | security | 无 | src 内联 input_validation tests（长度分支） | tests/e2e/security_rules_e2e.rs |
| SEC-04 | `strict()` vs `lenient()` 同一输入结论不同（如加密值/特殊模式放行差异） | 边界 | security | 无 | src 内联 input_validation tests | tests/e2e/security_rules_e2e.rs |
| SEC-05 | `sanitize_for_logging`：敏感值输出为掩码 | 正常 | security | 无 | src 内联 input_validation tests | tests/e2e/security_rules_e2e.rs |
| SEC-06 | 错误信息脱敏：`ConfersError` 展示链不含密钥/密码片段 | 正常 | security | 无 | `tests/security/security.rs::test_error_sanitization`；`⚠死文件 tests/core/error.rs::test_error_sanitized_chain/test_error_user_message_formatting` | tests/e2e/security_rules_e2e.rs |
| SEC-07 | `SecurityValidatorRegistry` 默认规则集验证一份完整 server 配置 | 正常 | security-rules | 无 | `tests/security/security.rs::test_registry_with_defaults_validates_all` | tests/e2e/security_rules_e2e.rs |
| SEC-08 | 一份坏配置（弱 JWT 密钥+错误 CORS+私网 SSRF）一次检出多个 `SecurityViolation` | 异常 | security-rules | 无 | `tests/security/security.rs::test_registry_detects_multiple_violations` | tests/e2e/security_rules_e2e.rs |
| SEC-09 | 干净配置全部通过，`SecurityReport` 无 violation | 正常 | security-rules | 无 | `tests/security/security.rs::test_registry_clean_config_passes` | tests/e2e/security_rules_e2e.rs |
| SEC-10 | fail_on_warning 模式：warning 级违规升级为失败 | 边界 | security-rules | 无 | `tests/security/security.rs::test_fail_on_warning_mode` | tests/e2e/security_rules_e2e.rs |
| SEC-11 | 自定义 `SecurityValidator` 注册后被 registry 调用 | 正常 | security-rules | 无 | `tests/security/security.rs::test_custom_validator_registration` | tests/e2e/security_rules_e2e.rs |
| SEC-12 | `CorsValidator`：通配 origin + credentials 组合判违规 | 异常 | security-rules | 无 | src 内联 `src/security/rules/cors.rs` tests | tests/e2e/security_rules_e2e.rs |
| SEC-13 | `SsrfValidator`：私网/链路本地地址（127.0.0.1/10.x/169.254.x）拒绝，配合 `remote::is_ip_blocked` | 异常 | security-rules,remote | 无 | src 内联 `src/security/rules/ssrf.rs`、`src/remote/poll.rs::is_ip_blocked` tests | tests/e2e/security_rules_e2e.rs |
| SEC-14 | `JwtSecretValidator`：短/常见弱密钥判 violation | 异常 | security-rules | 无 | src 内联 `src/security/rules/jwt.rs` tests | tests/e2e/security_rules_e2e.rs |
| SEC-15 | `TlsConfigValidator`：禁用校验/跳过 verify 判违规 | 异常 | security-rules | 无 | src 内联 `src/security/rules/tls.rs` tests | tests/e2e/security_rules_e2e.rs |
| SEC-16 | `ViolationSeverity` 排序与 `SecurityReport` 汇总（critical 优先） | 边界 | security-rules | 无 | src 内联 rules/mod tests | tests/e2e/security_rules_e2e.rs |
| SEC-17 | `config_injector`：把外部 env/secret 注入配置树指定前缀下 | 正常 | security | 无 | src 内联 `src/security/config_injector.rs` tests | tests/e2e/security_rules_e2e.rs |
| SEC-18 | 敏感键前缀识别（password/secret/token/api_key 等模式表 `patterns.rs` + `prefix.rs`）自动标记 sensitive | 边界 | security | 无 | src 内联 `src/security/patterns.rs/prefix.rs` tests | tests/e2e/security_rules_e2e.rs |

### 2.8 审计（AUD，11 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| AUD-01 | `AuditWriter::new()` 默认禁用：write 不产生文件、不报错 | 正常 | audit | 本地文件 | `tests/security/audit.rs::test_audit_writer_default` | tests/e2e/audit_e2e.rs |
| AUD-02 | `AuditWriterBuilder::enabled(true).log_dir(dir)` 构建后事件可写 | 正常 | audit | 本地文件 | `tests/security/audit.rs::test_audit_writer_builder` | tests/e2e/audit_e2e.rs |
| AUD-03 | best-effort 模式：`log_load/log_key_access` 事件持久化到日志文件 | 正常 | audit | 本地文件 | `tests/security/audit.rs::test_audit_best_effort_event_persists_to_file/test_audit_log_load/test_audit_log_key_access` | tests/e2e/audit_e2e.rs |
| AUD-04 | durable 模式：事件持久化且失败即报错 | 正常 | audit | 本地文件 | `tests/security/audit.rs::test_audit_durable_event_persists_to_file` | tests/e2e/audit_e2e.rs |
| AUD-05 | durable + 无效 log_dir → 返回错误（fail loud） | 异常 | audit | 本地文件 | `tests/security/audit.rs::test_audit_no_log_dir_returns_error_for_durable` | tests/e2e/audit_e2e.rs |
| AUD-06 | `log_decrypt(field, success/failure)` 输出脱敏（不含字段明文值） | 正常 | audit | 本地文件 | `tests/security/audit.rs::test_audit_log_decrypt_sanitizes` | tests/e2e/audit_e2e.rs |
| AUD-07 | `log_key_rotation(old_ver, new_ver)` 事件落盘 | 正常 | audit,key | 本地文件 | `tests/security/audit.rs::test_audit_log_key_rotation` | tests/e2e/audit_e2e.rs |
| AUD-08 | `AuditLevel::for_event` 事件→级别映射（load/access/decrypt/rotation） | 边界 | audit | 无 | `tests/security/audit.rs::test_audit_level_for_event` | tests/e2e/audit_e2e.rs |
| AUD-09 | `enabled(false)` 显式禁用：调用各 log_* 均为 no-op | 边界 | audit | 本地文件 | `tests/security/audit.rs::test_audit_disabled` | tests/e2e/audit_e2e.rs |
| AUD-10 | `AuditConfig/AuditConfigBuilder` 配置面（默认值/链式） | 正常 | audit | 无 | `tests/security/audit.rs::test_audit_config_default/test_audit_config_builder` | tests/e2e/audit_e2e.rs |
| AUD-11 | 多线程并发 write 同一日志文件：文件行数 = 写入数（无交错损坏） | 边界 | audit | 本地文件 | 无→需新增 | tests/e2e/audit_e2e.rs |

### 2.9 文件热更新（WAT，18 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| WAT-01 | `FsWatcher` 创建/启动/停止状态机（is_running 变化） | 正常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_fs_watcher_creation/test_fs_watcher_stop` | tests/e2e/watch_e2e.rs |
| WAT-02 | 修改被 watch 文件内容 → 事件通道收到变更 | 正常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_fs_watcher_file_modification_detection` | tests/e2e/watch_e2e.rs |
| WAT-03 | 新建被 watch 文件 → 检测到创建事件 | 正常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_fs_watcher_file_creation_detection` | tests/e2e/watch_e2e.rs |
| WAT-04 | 删除被 watch 文件 → 检测到删除事件且 watcher 不 panic（后续重载走错误路径） | 异常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_fs_watcher_file_deletion_detection` | tests/e2e/watch_e2e.rs |
| WAT-05 | watch 不存在的路径 → 启动报错 | 异常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_fs_watcher_nonexistent_path` | tests/e2e/watch_e2e.rs |
| WAT-06 | `MultiFsWatcher` 多文件同时 watch，逐一修改均收到事件 | 正常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_multi_fs_watcher_multiple_files/test_multi_fs_watcher_recv_detects_modification` | tests/e2e/watch_e2e.rs |
| WAT-07 | `MultiFsWatcher` 空 paths → 启动行为明确（空集/报错，按实现固化） | 边界 | watch | 本地文件 | `tests/watcher/watcher.rs::test_multi_fs_watcher_empty_paths` | tests/e2e/watch_e2e.rs |
| WAT-08 | `MultiFsWatcher` 含不存在路径 → 明确错误 | 异常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_multi_fs_watcher_nonexistent_paths` | tests/e2e/watch_e2e.rs |
| WAT-09 | `WatcherGuard` drop 后 watcher 停止（后台线程退出） | 正常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_watcher_guard_drop_stops_watcher/test_watcher_guard_start_stop/test_watcher_guard_shutdown` | tests/e2e/watch_e2e.rs |
| WAT-10 | `AdaptiveDebouncer`：窗口内重复变更只处理一次，窗口过后放行 | 边界 | watch | 本地文件 | `tests/watcher/watcher.rs::test_adaptive_debouncer_*`（6 例） | tests/e2e/watch_e2e.rs |
| WAT-11 | `min_reload_interval`：两次变更间隔小于阈值时第二次不触发重载 | 边界 | watch | 本地文件 | `tests/watcher/watcher.rs::test_watcher_config_with_min_reload_interval`（配置面）；行为级→需新增 | tests/e2e/watch_e2e.rs |
| WAT-12 | 连续失败达 `max_consecutive_failures` → 进入 `failure_pause` 暂停期 | 异常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_watcher_config_with_max_failures/test_watcher_config_with_failure_pause`（配置面）；行为级→需新增 | tests/e2e/watch_e2e.rs |
| WAT-13 | `rollback_on_validation_failure=true`：重载解析失败回滚到上一份好配置 | 异常 | watch,validation | 本地文件 | `tests/watcher/watcher.rs::test_watcher_config_with_rollback`（配置面）；行为级→需新增 | tests/e2e/watch_e2e.rs |
| WAT-14 | 无读权限文件 → 权限错误被报告且 watcher 不崩溃 | 异常 | watch | 本地文件 | `tests/watcher/watcher.rs::test_fs_watcher_permission_handling` | tests/e2e/watch_e2e.rs |
| WAT-15 | `WatcherConfig` 默认值 + builder 全链（debounce/min_interval/max_failures/pause/rollback） | 正常 | watch | 无 | `tests/watcher/watcher.rs::test_watcher_config_default/new/builder/builder_partial/builder_chaining/validation/clone/debug` | tests/e2e/watch_e2e.rs |
| WAT-16 | 端到端热更：改 TOML→重载→`DynamicField`/config 读到新值（对应示例 hot_reload） | 正常 | watch,dynamic | 本地文件 | examples/hot_reload；tests 级→需新增完整闭环 | tests/e2e/watch_e2e.rs |
| WAT-17 | 编辑器原子替换（写临时文件+rename 覆盖）触发且仅触发一次事件 | 边界 | watch | 本地文件 | 无→需新增（平台相关，Linux inotify） | tests/e2e/watch_e2e.rs |
| WAT-18 | 快速连续写 10 次（10ms 间隔）→ debounce 后至多重载 1-2 次（计数上限断言） | 边界 | watch | 本地文件 | 无→需新增（debounce 行为断言） | tests/e2e/watch_e2e.rs |

### 2.10 渐进重载（PGR，12 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| PGR-01 | Immediate 策略：健康即提交新配置 | 正常 | progressive-reload | 无 | `tests/core/progressive.rs::test_immediate_reload_commits` | tests/e2e/progressive_e2e.rs |
| PGR-02 | Immediate 提交原子性：`current()` 始终是完整旧值或完整新值（arc-swap） | 边界 | progressive-reload | 无 | `tests/core/progressive.rs::test_immediate_reload_atomic` | tests/e2e/progressive_e2e.rs |
| PGR-03 | Canary 策略：健康检查 Healthy → 提交 | 正常 | progressive-reload | 无 | `tests/core/progressive.rs::test_canary_reload_commits_when_healthy` | tests/e2e/progressive_e2e.rs |
| PGR-04 | Canary：Critical → 回滚旧配置 | 异常 | progressive-reload | 无 | `tests/core/progressive.rs::test_canary_reload_rollback_on_critical` | tests/e2e/progressive_e2e.rs |
| PGR-05 | Canary：Degraded → 继续推进不回滚 | 边界 | progressive-reload | 无 | `tests/core/progressive.rs::test_canary_reload_continues_on_degraded` | tests/e2e/progressive_e2e.rs |
| PGR-06 | Canary 无 health_check：默认放行提交 | 边界 | progressive-reload | 无 | `tests/core/progressive.rs::test_canary_reload_without_health_check` | tests/e2e/progressive_e2e.rs |
| PGR-07 | Linear 策略：分步依赖就绪后逐步提交 | 正常 | progressive-reload | 无 | `tests/core/progressive.rs::test_linear_reload_commits_after_steps` | tests/e2e/progressive_e2e.rs |
| PGR-08 | 自定义 `ReloadHealthCheck` 拒绝无效新配置（真实校验函数） | 异常 | progressive-reload,validation | 无 | `tests/core/progressive.rs::test_real_health_check_validates_config/test_real_health_check_critical_on_invalid` | tests/e2e/progressive_e2e.rs |
| PGR-09 | `ProgressiveReloader` Clone 语义与 `current()` 返回 Arc 快照 | 正常 | progressive-reload | 无 | `tests/core/progressive.rs::test_progressive_reloader_is_clone/test_current_returns_arc` | tests/e2e/progressive_e2e.rs |
| PGR-10 | Builder：initial/strategy/health_check/with_dependencies 链式构建 | 正常 | progressive-reload | 无 | `tests/core/progressive.rs::test_builder_default/test_with_dependencies` | tests/e2e/progressive_e2e.rs |
| PGR-11 | `ReloadOutcome`/`HealthStatus` 变体完备（Committed/RolledBack 等 × Healthy/Degraded/Critical） | 边界 | progressive-reload | 无 | `tests/core/progressive.rs::test_reload_outcome_variants/test_health_status_variants` | tests/e2e/progressive_e2e.rs |
| PGR-12 | ConfigProvider 适配：`config.health_check()` Degraded 上报（含 lifecycle 注册路径） | 边界 | progressive-reload | 无 | `tests/core/progressive.rs::test_config_provider_implementation/test_config_health_check_degraded` | tests/e2e/progressive_e2e.rs |

### 2.11 动态字段（DYN，14 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| DYN-01 | `DynamicField` 初值读取（get/get_ref，含字符串与数值） | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_get_returns_initial/test_dynamic_field_get_returns_initial_string/test_dynamic_field_get_ref` | tests/e2e/dynamic_e2e.rs |
| DYN-02 | `update()` 新值生效并触发回调（回调收到新值） | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_update/test_dynamic_field_callback_registration` | tests/e2e/dynamic_e2e.rs |
| DYN-03 | 同一字段注册多个回调全部被调用 | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_multiple_callbacks/test_dynamic_field_callback_count_tracking` | tests/e2e/dynamic_e2e.rs |
| DYN-04 | 无回调注册时 update 静默成功 | 边界 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_no_callbacks` | tests/e2e/dynamic_e2e.rs |
| DYN-05 | `CallbackGuard` drop 自动注销回调 | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_callback_guard_deregisters_on_drop/test_callback_guard_drops_on_scope_exit` | tests/e2e/dynamic_e2e.rs |
| DYN-06 | 多 guard 各自独立注销，互不影响 | 边界 | dynamic | 无 | `tests/core/dynamic.rs::test_multiple_callback_guards` | tests/e2e/dynamic_e2e.rs |
| DYN-07 | 异步回调（async callback）被调度执行 | 正常 | dynamic,tokio | 无 | `tests/core/dynamic.rs::test_dynamic_field_async_callback` | tests/e2e/dynamic_e2e.rs |
| DYN-08 | 复杂类型字段（嵌套 struct）动态更新 | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_complex_type` | tests/e2e/dynamic_e2e.rs |
| DYN-09 | `DynamicFieldBuilder` 链式构建与默认实现 | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_builder/test_dynamic_field_builder_default/test_dynamic_field_default` | tests/e2e/dynamic_e2e.rs |
| DYN-10 | `FieldWatcher`（dynamic+watch）：字段值变化才触发 | 正常 | dynamic,watch | 无 | `tests/core/dynamic.rs::test_field_watcher_changed_for` | tests/e2e/dynamic_e2e.rs |
| DYN-11 | `FieldWatcher`：字段未变不触发回调 | 边界 | dynamic,watch | 无 | `tests/core/dynamic.rs::test_field_watcher_no_trigger_if_field_unchanged` | tests/e2e/dynamic_e2e.rs |
| DYN-12 | 多线程并发 update + read 无数据竞争（锁策略验证） | 边界 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_multithread_callbacks` | tests/e2e/dynamic_e2e.rs |
| DYN-13 | 与真实 `ConfigConnector` 集成：config set 后 dynamic 字段反映 | 正常 | dynamic | 无 | `tests/core/dynamic.rs::test_dynamic_field_with_real_config/test_callback_with_real_config/test_real_config_provider` | tests/e2e/dynamic_e2e.rs |
| DYN-14 | 派生宏 `#[config(dynamic = true)]` 字段生成 handle，重载后 handle 读到新值 | 正常 | dynamic,macros | 本地文件 | src 内联 `macros/src/codegen` 相关 + examples/dynamic_fields；集成级→需新增 | tests/e2e/dynamic_e2e.rs |

### 2.12 特性开关（TGL，6 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| TGL-01 | register→enable→disable→toggle→is_enabled 全生命周期状态正确 | 正常 | feature-toggle | 无 | `tests/core/toggle.rs::test_toggle_lifecycle` | tests/e2e/toggle_e2e.rs |
| TGL-02 | 对未注册 toggle enable/disable/toggle/is_enabled → 返回 false（不 panic） | 异常 | feature-toggle | 无 | `tests/core/toggle.rs::test_toggle_lifecycle`（部分）+ src 内联 toggle tests→需显式场景 | tests/e2e/toggle_e2e.rs |
| TGL-03 | `load_from_config(provider, "features")`：从配置树批量初始化开关 | 正常 | feature-toggle | 本地文件 | `tests/core/toggle.rs::test_load_from_config_integration` | tests/e2e/toggle_e2e.rs |
| TGL-04 | 多线程并发 enable/disable/toggle 最终状态一致（dashmap 并发安全） | 边界 | feature-toggle | 无 | `tests/core/toggle.rs::test_concurrent_toggle_operations` | tests/e2e/toggle_e2e.rs |
| TGL-05 | `list()` 返回全部 FeatureInfo（名称/描述/状态），len/is_empty 一致 | 正常 | feature-toggle | 无 | `tests/core/toggle.rs::test_list_returns_all_toggles` | tests/e2e/toggle_e2e.rs |
| TGL-06 | toggle 控制 dynamic 字段/回调启停（dynamic+toggle 组合入口） | 边界 | feature-toggle,dynamic | 无 | 无→需新增 | tests/e2e/combo_e2e.rs（CMP-03） |

### 2.13 版本迁移（MIG，12 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| MIG-01 | `Versioned` trait：version() 返回声明版本；不同 struct 不同版本 | 正常 | migration | 无 | `tests/core/migration.rs::test_versioned_trait_implementation/test_versioned_different_versions` | tests/e2e/migration_e2e.rs |
| MIG-02 | `MigrationRegistry::register(v1,v2,fn)` 注册并返回 self（builder 链） | 正常 | migration | 无 | `tests/core/migration.rs::test_registry_register/test_registry_register_returns_self/test_registry_builder_pattern/test_registry_with_migrations` | tests/e2e/migration_e2e.rs |
| MIG-03 | 直接迁移 v1→v2 数据转换正确 | 正常 | migration | 无 | `tests/core/migration.rs::test_migrate_direct` | tests/e2e/migration_e2e.rs |
| MIG-04 | 链式迁移 v1→v3（经 v2）按序执行 | 正常 | migration | 无 | `tests/core/migration.rs::test_migrate_chain/test_precompute_paths_chain` | tests/e2e/migration_e2e.rs |
| MIG-05 | 同版本迁移 → no-op 直接返回 | 边界 | migration | 无 | `tests/core/migration.rs::test_migrate_same_version` | tests/e2e/migration_e2e.rs |
| MIG-06 | 无迁移路径（v1→v9 未注册）→ `MigrationError`（2600 版本不匹配族） | 异常 | migration | 无 | `tests/core/migration.rs::test_migrate_no_path`；`⚠死文件 tests/core/error.rs::test_migration_error` | tests/e2e/migration_e2e.rs |
| MIG-07 | 迁移 fn 内部出错 → 错误向调用方传播，不产出半成品配置 | 异常 | migration | 无 | `tests/core/migration.rs::test_migrate_failure` | tests/e2e/migration_e2e.rs |
| MIG-08 | `precompute_paths` 复杂图（多分支）选路正确；direct 路径优先于链 | 边界 | migration | 无 | `tests/core/migration.rs::test_precompute_paths_direct/complex_graph/direct_preferred_over_chain/no_path` | tests/e2e/migration_e2e.rs |
| MIG-09 | `MigrationOnReload` 三态（Skip/Always/IfChanged）语义与默认值 | 边界 | migration | 无 | `tests/core/migration.rs::test_migration_on_reload_variants/default/clone/debug` | tests/e2e/migration_e2e.rs |
| MIG-10 | 派生宏 `ConfigMigration` 生成 Versioned 实现（version 属性透传） | 正常 | migration,macros | 无 | `tests/core/derive.rs::test_config_migration_derive_generates_versioned` | tests/e2e/migration_e2e.rs |
| MIG-11 | migration+snapshot 组合：旧版本配置迁移后快照记录的是新版本内容 | 边界 | migration,snapshot | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs（CMP-02） |
| MIG-12 | migration+watch 组合：热更文件被改回旧版本 → 自动迁移后生效 | 边界 | migration,watch | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs（CMP-05） |

### 2.14 快照（SNP，10 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| SNP-01 | `SnapshotManager::save` JSON 格式落盘，文件名含时间戳 | 正常 | snapshot | 本地文件 | src 内联 `src/impl_/snapshot.rs` tests；examples/snapshot | tests/e2e/snapshot_e2e.rs |
| SNP-02 | `SnapshotFormat::{Toml,Yaml}` 各格式 save→load roundtrip | 正常 | snapshot | 本地文件 | src 内联 snapshot tests（`SnapshotFormat::ext`） | tests/e2e/snapshot_e2e.rs |
| SNP-03 | `list_snapshots` 按时间排序返回 `SnapshotInfo` | 正常 | snapshot | 本地文件 | src 内联 snapshot tests | tests/e2e/snapshot_e2e.rs |
| SNP-04 | `load_snapshot(path)` 恢复为 `AnnotatedValue` 与保存时等价 | 正常 | snapshot | 本地文件 | src 内联 snapshot tests | tests/e2e/snapshot_e2e.rs |
| SNP-05 | `prune_old_snapshots` 清理过期快照并返回数量 | 正常 | snapshot | 本地文件 | src 内联 snapshot tests | tests/e2e/snapshot_e2e.rs |
| SNP-06 | 快照目录不存在时 list/prune → 空列表/明确处理（不 panic） | 边界 | snapshot | 本地文件 | `tests/cli/commands.rs::test_snapshot_list_empty`（CLI 面）；库面→需新增 | tests/e2e/snapshot_e2e.rs |
| SNP-07 | `load_snapshot(不存在文件)` → 错误 | 异常 | snapshot | 本地文件 | src 内联 snapshot tests→需显式场景 | tests/e2e/snapshot_e2e.rs |
| SNP-08 | 损坏快照（写入垃圾内容）load → 解析错误 | 异常 | snapshot | 本地文件 | 无→需新增 | tests/e2e/snapshot_e2e.rs |
| SNP-09 | `SnapshotConfig` max 数量上限：超限滚动删除最旧（保存 N+1 份后最旧消失） | 边界 | snapshot | 本地文件 | src 内联 snapshot tests→需集成新增 | tests/e2e/snapshot_e2e.rs |
| SNP-10 | `ConfigBuilder::with_snapshot` 构建时自动快照端到端（构建→快照文件存在→可回放） | 正常 | snapshot | 本地文件 | src 内联 builder with_snapshot tests→需集成新增 | tests/e2e/snapshot_e2e.rs |

### 2.15 模块 / 配置分组（MOD，11 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| MOD-01 | `ModuleConfig::new(name, [(profile,path)], default)` 注册多 profile；profiles/has_profile/get_profile 正确 | 正常 | modules | 本地文件 | `tests/core/modules.rs::test_module_config_new/get_profile/has_profile/profiles` | tests/e2e/modules_e2e.rs |
| MOD-02 | default profile 推断：有 default 时用它，无 default 时取第一个 | 边界 | modules | 本地文件 | `tests/core/modules.rs::test_module_config_default_profile_with_default/without_default` | tests/e2e/modules_e2e.rs |
| MOD-03 | `set_active_profile`：合法切换成功；非法 profile/group → 错误 | 异常 | modules | 本地文件 | `tests/core/modules.rs::test_set_active_profile/nonexistent_profile/nonexistent_group` | tests/e2e/modules_e2e.rs |
| MOD-04 | `load_module` 加载 toml/json profile 文件成功 | 正常 | modules | 本地文件 | `tests/core/modules.rs::test_load_module_success_toml/test_load_module_success_json` | tests/e2e/modules_e2e.rs |
| MOD-05 | `load_module(group 不存在)` → `ModuleNotFound` 错误 | 异常 | modules | 本地文件 | `tests/core/modules.rs::test_load_module_not_found_group`；`⚠死文件 tests/core/error.rs::test_module_not_found_error` | tests/e2e/modules_e2e.rs |
| MOD-06 | `load_module(profile 不存在)` → 错误 | 异常 | modules | 本地文件 | `tests/core/modules.rs::test_load_module_not_found_profile` | tests/e2e/modules_e2e.rs |
| MOD-07 | `load_module(文件不存在)` → 文件错误而非 panic | 异常 | modules | 本地文件 | `tests/core/modules.rs::test_load_module_file_not_found` | tests/e2e/modules_e2e.rs |
| MOD-08 | `load_active`：按当前激活 profile 加载；无激活 → 错误 | 正常/异常 | modules | 本地文件 | `tests/core/modules.rs::test_load_active_success/test_load_active_not_found` | tests/e2e/modules_e2e.rs |
| MOD-09 | `ModuleRegistry` 多 group 注册、list_groups、chaining、get/contains/len | 正常 | modules | 本地文件 | `tests/core/modules.rs::test_register_group/register_multiple_groups/list_groups/registry_chaining/multiple_groups_load/get_module_config/registry_with_capacity` | tests/e2e/modules_e2e.rs |
| MOD-10 | 派生宏 `ConfigModules` 生成 registry（module_group 属性） | 正常 | modules,macros | 无 | `tests/core/derive.rs::test_config_modules_derive_generates_registry` | tests/e2e/modules_e2e.rs |
| MOD-11 | profile_env（APP_ENV）驱动激活 profile 切换（结构属性 profile/profile_env 端到端） | 边界 | modules,macros | 本地文件 | src 内联 `macros/src/codegen/modules.rs`；集成级→需新增 | tests/e2e/modules_e2e.rs |

### 2.16 上下文感知（CTX，9 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| CTX-01 | `EvaluationContext` 默认/with_key/with_attributes/clone/debug | 正常 | context-aware | 无 | `tests/core/context.rs::test_evaluation_context_default/with_key/with_attributes/clone/debug` | tests/e2e/context_e2e.rs |
| CTX-02 | `ContextRule` 谓词匹配 → `ContextAwareField` 返回规则值 | 正常 | context-aware | 无 | `tests/core/context.rs::test_context_aware_field_with_matching_predicate` | tests/e2e/context_e2e.rs |
| CTX-03 | 无规则匹配 → 返回 default 值 | 边界 | context-aware | 无 | `tests/core/context.rs::test_context_aware_field_no_match_returns_default` | tests/e2e/context_e2e.rs |
| CTX-04 | 多规则按序求值：第一条命中即返回（优先序固化） | 边界 | context-aware | 无 | `tests/core/context.rs::test_context_aware_field_multiple_rules` | tests/e2e/context_e2e.rs |
| CTX-05 | `evaluate` 返回引用语义（`evaluate_returns_reference`） | 正常 | context-aware | 无 | `tests/core/context.rs::test_context_aware_field_evaluate_returns_reference` | tests/e2e/context_e2e.rs |
| CTX-06 | environment/region 维度上下文（dev/prod、cn/us）切换取值 | 正常 | context-aware | 无 | `tests/core/context.rs::test_evaluation_context_environment/region` | tests/e2e/context_e2e.rs |
| CTX-07 | `test_upload_limit_use_case` 同构端到端：按 region 限流上传上限 | 正常 | context-aware | 无 | `tests/core/context.rs::test_upload_limit_use_case` | tests/e2e/context_e2e.rs |
| CTX-08 | `ContextAwareField` Send+Sync（跨线程求值） | 边界 | context-aware | 无 | `tests/core/context.rs::test_context_aware_field_send_sync` | tests/e2e/context_e2e.rs |
| CTX-09 | context-aware + dynamic 组合：上下文变化触发热值切换 | 边界 | context-aware,dynamic | 无 | 无→需新增 | tests/e2e/combo_e2e.rs（CMP-17） |

### 2.17 HTTP 轮询远程源（REM，11 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| REM-01 | `HttpPolledSourceBuilder` 无 url → `build()` 报错 | 异常 | remote | 无 | `tests/remote/remote.rs::test_builder_requires_url` | tests/e2e/remote_e2e.rs |
| REM-02 | builder 全链：url/interval/format/timeout/allowed_domain(s) | 正常 | remote | 无 | `tests/remote/remote.rs::test_builder_accepts_url/custom_interval/with_format/with_timeout/builder_pattern` | tests/e2e/remote_e2e.rs |
| REM-03 | 默认轮询间隔值固化（`test_default_poll_interval`） | 边界 | remote | 无 | `tests/remote/remote.rs::test_default_poll_interval` | tests/e2e/remote_e2e.rs |
| REM-04 | 各种 URL 形式（host:port、https://、带 path）被接受 | 正常 | remote | 无 | `tests/remote/remote.rs::test_various_url_formats` | tests/e2e/remote_e2e.rs |
| REM-05 | SSRF 防护：明文 http URL 拒绝 | 异常 | remote | 无 | `tests/remote/remote.rs::test_http_url_rejected` | tests/e2e/remote_e2e.rs |
| REM-06 | SSRF 防护：私网 IP 目标拒绝（配合 `is_ip_blocked`） | 异常 | remote | 无 | `tests/remote/remote.rs::test_private_ip_rejected` | tests/e2e/remote_e2e.rs |
| REM-07 | 非法/空 URL → build 错误 | 异常 | remote | 无 | `tests/remote/remote.rs::test_invalid_url_rejected/test_empty_url_rejected` | tests/e2e/remote_e2e.rs |
| REM-08 | `PolledSource` trait object 可装箱（dyn 兼容） | 正常 | remote | 无 | `tests/remote/remote.rs::test_trait_object` | tests/e2e/remote_e2e.rs |
| REM-09 | 轮询目标不可达 → 错误/重试路径，circuit_breaker_threshold 触发后快速失败，退避到 max_delay | 异常 | remote | 无（不可达地址） | src 内联 `src/remote/circuit_breaker.rs` tests；轮询级→需新增 | tests/e2e/remote_e2e.rs |
| REM-10 | 熔断半开恢复：故障恢复后源重新可用（真实 HTTP 端点，可用 compose 内服务代替） | 边界 | remote | 本地文件（用 Consul 冒充 HTTP 端点亦可） | 无→需新增 | tests/e2e/remote_e2e.rs |
| REM-11 | 轮询内容变化 → 新配置生效（端到端 poll 循环） | 正常 | remote | Consul(8500)（作 HTTP KV 端点） | 无→需新增 | tests/e2e/remote_e2e.rs |

### 2.18 etcd（ETC，10 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| ETC-01 | `EtcdSourceBuilder::endpoint("127.0.0.1:2379")` 连接真实 etcd 成功 | 正常 | etcd | etcd(2379) | `tests/remote/etcd.rs::test_etcd_source_connect`（有服务才跑，`tests/common.rs::is_service_available` 守卫） | tests/e2e/remote_e2e.rs |
| ETC-02 | 用户名/密码认证连接 | 正常 | etcd | etcd(2379) | `tests/remote/etcd.rs::test_etcd_source_with_auth` | tests/e2e/remote_e2e.rs |
| ETC-03 | 写入 TOML KV → source 解析为配置树；JSON 形态亦然 | 正常 | etcd,toml,json | etcd(2379) | `tests/remote/etcd.rs::test_etcd_source_parse_config/test_etcd_source_json_config` | tests/e2e/remote_e2e.rs |
| ETC-04 | 认证信息从环境变量读取（`test_etcd_auth_env_var` 语义） | 正常 | etcd | etcd(2379) | `tests/remote/remote.rs::test_etcd_auth_env_var` | tests/e2e/remote_e2e.rs |
| ETC-05 | 非法 endpoint 字符串 → 构造错误 | 异常 | etcd | 无 | `tests/remote/remote.rs::test_etcd_invalid_endpoint` | tests/e2e/remote_e2e.rs |
| ETC-06 | etcd 不可达（127.0.0.1:1）→ 连接错误 | 异常 | etcd | 无（不可达地址） | `tests/remote/remote.rs::test_etcd_connection_failure` | tests/e2e/remote_e2e.rs |
| ETC-07 | TLS 连接配置（tls 分支构造） | 边界 | etcd | etcd(2379) | `tests/remote/remote.rs::test_etcd_tls_connection/test_etcd_builder_tls` | tests/e2e/remote_e2e.rs |
| ETC-08 | builder 链 endpoint/auth/prefix/interval/tls chaining | 正常 | etcd | 无 | `tests/remote/remote.rs::test_etcd_builder_*`（6 例） | tests/e2e/remote_e2e.rs |
| ETC-09 | 端到端：写 KV→轮询读→改 KV→再读到新值（远程热更语义） | 正常 | etcd,remote | etcd(2379) | 无→需新增 | tests/e2e/remote_e2e.rs |
| ETC-10 | 并发写多个 key 后轮询读取：prefix 树完整（无丢键/半写状态） | 边界 | etcd | etcd(2379) | 无→需新增 | tests/e2e/remote_e2e.rs |

### 2.19 Consul（CSL，8 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| CSL-01 | `ConsulSourceBuilder::address("127.0.0.1:8500")` 连接真实 Consul | 正常 | consul | Consul(8500) | `tests/remote/consul.rs::test_consul_source_connect`（服务守卫） | tests/e2e/remote_e2e.rs |
| CSL-02 | 带 token 访问受保护 KV（dev 模式默认 token 语义固化） | 正常 | consul | Consul(8500) | `tests/remote/consul.rs::test_consul_source_with_token` | tests/e2e/remote_e2e.rs |
| CSL-03 | prefix 下 KV 树解析为配置 | 正常 | consul | Consul(8500) | `tests/remote/consul.rs::test_consul_source_parse_config` | tests/e2e/remote_e2e.rs |
| CSL-04 | Consul 不可达 → 连接错误 | 异常 | consul | 无（不可达地址） | `tests/remote/remote.rs::test_consul_connection_failure` | tests/e2e/remote_e2e.rs |
| CSL-05 | tls_skip_verify（debug）分支构造 | 边界 | consul | Consul(8500) | `tests/remote/remote.rs::test_consul_tls_config/test_consul_builder_tls_skip_verify_debug` | tests/e2e/remote_e2e.rs |
| CSL-06 | builder 链 address/token/prefix/interval | 正常 | consul | 无 | `tests/remote/remote.rs::test_consul_builder_*`（5 例） | tests/e2e/remote_e2e.rs |
| CSL-07 | 端到端：Consul 写 KV → poll 读到新值 → 删除 KV → 处理空配置 | 正常 | consul,remote | Consul(8500) | 无→需新增 | tests/e2e/remote_e2e.rs |
| CSL-08 | DoS 防护：`max_response_bytes`/`max_kv_entries` 超限拒绝 | 边界 | consul | Consul(8500) | src 内联 consul 限制 tests（examples/remote_consul 演示）；集成级→需新增 | tests/e2e/remote_e2e.rs |

### 2.20 配置总线 — 进程内（BUS，10 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| BUS-01 | `InMemoryBus::new/with_capacity` 创建与 `subscriber_count` | 正常 | config-bus | 无 | `tests/remote/bus.rs::test_in_memory_bus_new/test_in_memory_bus_with_capacity` | tests/e2e/bus_e2e.rs |
| BUS-02 | `publish` → `subscribe` 收到 `ConfigChangeEvent`（instance/source/keys/checksum） | 正常 | config-bus | 无 | `tests/remote/bus.rs::test_in_memory_bus_publish_subscribe/test_config_change_event_creation` | tests/e2e/bus_e2e.rs |
| BUS-03 | 多订阅者广播：一条事件全部收到 | 正常 | config-bus | 无 | `tests/remote/bus.rs::test_in_memory_bus_multiple_subscribers` | tests/e2e/bus_e2e.rs |
| BUS-04 | 容量满（capacity=1 连发多条）：滞后订阅者行为明确（丢旧/阻塞，按实现固化）不 panic | 边界 | config-bus | 无 | 无→需新增 | tests/e2e/bus_e2e.rs |
| BUS-05 | `BusBuilder::capacity(n).build()` 链式构建 | 正常 | config-bus | 无 | `tests/remote/bus.rs::test_bus_builder` | tests/e2e/bus_e2e.rs |
| BUS-06 | `ConfigChangeEvent` 序列化 roundtrip（serde） | 正常 | config-bus | 无 | `tests/remote/bus.rs::test_event_serialization` | tests/e2e/bus_e2e.rs |
| BUS-07 | `BusEventLimiter`：`try_acquire` 限流、`release/reset` 语义 | 边界 | config-bus | 无 | src 内联 `src/bus/limiter.rs` tests | tests/e2e/bus_e2e.rs |
| BUS-08 | `ConfigBuilder::config_bus(Arc<dyn ConfigBus>)` 集成：文件变更事件驱动应用重载 | 正常 | config-bus,watch | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs（CMP-04） |
| BUS-09 | `Lifecycle` start/stop 后 publish/subscribe 状态明确（stop 后 publish 报错或忽略） | 边界 | config-bus | 无 | `tests/remote/bus.rs` 生命周期相关（nats/redis start_stop 为真实服务版）；InMemory 级→需新增 | tests/e2e/bus_e2e.rs |
| BUS-10 | 订阅者 receiver 被 drop 后 publish 不 panic，subscriber_count 递减 | 边界 | config-bus | 无 | 无→需新增 | tests/e2e/bus_e2e.rs |

### 2.21 NATS 总线（NAT，7 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| NAT-01 | `NatsConfigBus::connect("nats://127.0.0.1:4222", subject)` 连接真实 NATS | 正常 | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_connect/test_nats_bus_connect_direct`（端口守卫，无服务时跳过） | tests/e2e/bus_e2e.rs |
| NAT-02 | 连接不可达 NATS（错误端口）→ 连接错误不 panic | 异常 | nats-bus | 无（不可达地址） | `tests/remote/bus.rs::test_nats_bus_connect`（失败分支） | tests/e2e/bus_e2e.rs |
| NAT-03 | `NatsBusBuilder` url/subject/stream_name/options → build | 正常 | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_url_method/test_nats_bus_subject/test_nats_bus_with_stream_name/test_nats_bus_options` | tests/e2e/bus_e2e.rs |
| NAT-04 | start/stop 生命周期（JetStream stream 建立与清理） | 正常 | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_start_stop` | tests/e2e/bus_e2e.rs |
| NAT-05 | 端到端发布/订阅（跨实例：两个 bus 实例互发） | 正常 | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_publish_subscribe` | tests/e2e/bus_e2e.rs |
| NAT-06 | 总线断连：测试中停掉 NATS 容器/断开连接 → 错误上报，恢复后可重新收发 | 异常 | nats-bus | NATS(4222) | 无→需新增 | tests/e2e/bus_e2e.rs |
| NAT-07 | nats-bus 注入 `ConfigBuilder::config_bus`：实例 A 改配置 → 实例 B 收到事件重载 | 边界 | nats-bus,watch | NATS(4222) | 无→需新增 | tests/e2e/combo_e2e.rs（CMP-16） |

### 2.22 Redis 总线（RDS，7 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| RDS-01 | `RedisConfigBus::connect("redis://127.0.0.1:16379", channel)` 连接真实 Redis | 正常 | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_connect/test_redis_bus_default`（端口守卫） | tests/e2e/bus_e2e.rs |
| RDS-02 | 连接不可达 Redis → 连接错误 | 异常 | redis-bus | 无（不可达地址） | `tests/remote/bus.rs::test_redis_bus_connect`（失败分支） | tests/e2e/bus_e2e.rs |
| RDS-03 | `RedisBusBuilder` url/channel/pool_size/retry_wait_ms/error_retry_wait_secs → build | 正常 | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_url_method/with_pool_size/pool_size_method` | tests/e2e/bus_e2e.rs |
| RDS-04 | start/stop 生命周期 | 正常 | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_start_stop` | tests/e2e/bus_e2e.rs |
| RDS-05 | 端到端 pub/sub（pub/sub 端到端收发事件） | 正常 | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_publish_subscribe` | tests/e2e/bus_e2e.rs |
| RDS-06 | 连接池 pool_size>1 并发发布不丢消息 | 边界 | redis-bus | Redis(16379) | 无→需新增 | tests/e2e/bus_e2e.rs |
| RDS-07 | 总线断连恢复：中途 flushall/断连 → 按 error_retry_wait_secs 重试后恢复 | 异常 | redis-bus | Redis(16379) | 无→需新增 | tests/e2e/bus_e2e.rs |

### 2.23 Schema（SCH，5 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| SCH-01 | `ConfigSchema` 派生生成 JSON Schema：类型/必填/默认值正确 | 正常 | schema,macros | 无 | `tests/core/derive.rs::test_config_schema_derive_generates_json_schema` | tests/e2e/schema_e2e.rs |
| SCH-02 | `TypeScriptGenerator::generate::<T>()` 生成 TS 类型声明，嵌套结构映射 interface | 正常 | typescript-schema,macros | 无 | `tests/core/derive.rs::test_config_schema_derive_generates_typescript_type`、src 内联 `src/impl_/schema.rs` tests | tests/e2e/schema_e2e.rs |
| SCH-03 | `typescript-schema` 是 `schema` 的 alias：开 alias 即得全部 schema 能力 | 边界 | typescript-schema | 无 | Cargo.toml 声明核对 + PRS 编译矩阵 | tests/e2e/presets_e2e.rs |
| SCH-04 | 嵌套 struct/Option/Vec 字段的 schema 输出正确性 | 边界 | schema | 无 | src 内联 schema tests→需新增断言 | tests/e2e/schema_e2e.rs |
| SCH-05 | schema 与 CLI validate 联动：按 schema 语义校验一份坏配置报 issue | 异常 | schema,cli | 本地文件 | `tests/cli/commands.rs::test_validate_command`（启发式校验） | tests/e2e/cli_e2e.rs |

### 2.24 CLI（CLI，24 条，二进制 `confers`，`cargo run --features cli -- …` 或 assert_cmd 形态）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| CLI-01 | `inspect -c app.toml` 文本表格输出 KEY/VALUE/SOURCE/LOCATION | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_inspect_command` | tests/e2e/cli_e2e.rs |
| CLI-02 | `inspect --format json` 输出可解析 JSON（溯源树） | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_inspect_json_output` | tests/e2e/cli_e2e.rs |
| CLI-03 | `inspect -k server.port` 过滤；不存在的键输出 `[NOT FOUND]` 且退出码 0 | 正常/边界 | cli | 本地文件 | src 内联 `src/cli/mod.rs::test_find_value_by_key_missing`；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-04 | `inspect --show-conflicts`：被覆盖键带 `*` 冲突标记 | 边界 | cli | 本地文件 | src 内联 `test_print_config_value_conflict_marker_when_priority`；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-05 | `validate` 合法配置 → 成功 + "All validation checks passed" | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_validate_command` | tests/e2e/cli_e2e.rs |
| CLI-06 | `validate --strict` 有 issue 时非零退出码 | 异常 | cli | 本地文件 | src 内联 strict 分支测试；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-07 | `validate --format json` 输出 `{"valid":…,"issues":[…]}` 结构 | 正常/异常 | cli | 本地文件 | `tests/cli/commands.rs::test_validate_json_output` | tests/e2e/cli_e2e.rs |
| CLI-08 | `export --format json/toml` 输出合并后配置 | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_export_command_json/test_export_command_toml` | tests/e2e/cli_e2e.rs |
| CLI-09 | `export -o 文件` 写文件；`-o 目录` 自动时间戳文件名 | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_export_to_file` | tests/e2e/cli_e2e.rs |
| CLI-10 | `export --with-provenance` 输出含 source/location 的 annotated 树 | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_export_with_provenance` | tests/e2e/cli_e2e.rs |
| CLI-11 | `export --raw` 向 stderr 打印未脱敏警告 | 边界 | cli | 本地文件 | src 内联 raw 警告分支；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-12 | `diff --base a.toml --overlay b.toml` 输出 unified diff；`--format json` 输出结构化 | 正常 | cli | 本地文件 | `tests/cli/commands.rs::test_diff_command/test_diff_json_output` | tests/e2e/cli_e2e.rs |
| CLI-13 | `diff` 内容相同的两文件 → "Configurations are identical" | 边界 | cli | 本地文件 | `tests/cli/commands.rs::test_diff_identical_configs` | tests/e2e/cli_e2e.rs |
| CLI-14 | `diff` 传绝对路径 → bail 提示；加 `--allow-absolute-paths` 后成功 | 异常 | cli | 本地文件 | src 内联绝对路径 bail 分支测试；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-15 | `snapshot list --directory`：空目录/含 json+toml 快照两种输出 | 正常/边界 | cli | 本地文件 | `tests/cli/commands.rs::test_snapshot_list_empty`；非空目录级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-16 | `snapshot diff --latest 2`：快照不足 2 份时友好提示退出 | 异常 | cli | 本地文件 | src 内联 `cmd_snapshot_diff` 不足分支；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-17 | `snapshot prune --older-than abc` → fail loud：invalid duration 错误 | 异常 | cli | 本地文件 | src 内联 `cmd_snapshot_prune` 解析失败分支；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-18 | `--env-file .env` 加载变量参与配置；文件不存在 → 报错退出 | 正常/异常 | cli,env | 本地文件 | `tests/cli/commands.rs::test_env_file_loading`、src 内联 `test_load_env_file_not_found` | tests/e2e/cli_e2e.rs |
| CLI-19 | `--env-file` 超 1 MiB 文件 → "size limit" 错误 | 异常 | cli,env | 本地文件 | src 内联 `test_load_env_file_rejects_oversized_file` | tests/e2e/cli_e2e.rs |
| CLI-20 | `--env-file` 单行超 16 KiB → "line too long" 错误 | 异常 | cli,env | 本地文件 | src 内联 `test_load_env_file_rejects_oversized_line` | tests/e2e/cli_e2e.rs |
| CLI-21 | `--help`/`--version` 输出子命令清单与版本号 | 正常 | cli | 无 | `tests/cli/commands.rs::test_help_flag/test_version_flag`、`tests/cli/help.rs`（6 例） | tests/e2e/cli_e2e.rs |
| CLI-22 | `-c 不存在路径`：路径被跳过（仅 env 参与构建）不报错 | 边界 | cli | 本地文件 | src 内联 `test_build_config_from_cli_nonexistent_path_skipped` | tests/e2e/cli_e2e.rs |
| CLI-23 | `export --format xml` → "Unsupported format" bail 非零退出 | 异常 | cli | 本地文件 | src 内联 unsupported format 分支；CLI 级→需新增 | tests/e2e/cli_e2e.rs |
| CLI-24 | inspect 对 >20 字符长字符串截断显示，UTF-8 多字节（中文）不 panic 按字符截断 | 边界 | cli | 本地文件 | src 内联 `test_format_value_long_string_truncation` 等 | tests/e2e/cli_e2e.rs |

### 2.25 派生宏与属性（MAC，18 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| MAC-01 | `#[derive(Config)]` 无属性 struct：编译并生成加载代码，`test_simple_config_default` 同构 | 正常 | macros | 无 | `tests/core/derive.rs::test_simple_config_default` | tests/e2e/macro_e2e.rs |
| MAC-02 | 字段 env 映射：`field_name` → `FIELD_NAME` 环境变量覆盖 | 正常 | macros,env | 无 | `tests/core/derive.rs::test_simple_config_env_mapping` | tests/e2e/macro_e2e.rs |
| MAC-03 | `name` / `name_env` 字段属性覆盖默认键名/env 名 | 边界 | macros | 本地文件 | src 内联 `macros/src/parse.rs::effective_name/effective_env_name` tests；集成级→需新增 | tests/e2e/macro_e2e.rs |
| MAC-04 | `Option<T>` 字段缺省为 None 不报 MissingField | 边界 | macros | 本地文件 | `tests/core/derive.rs::test_optional_config_default` | tests/e2e/macro_e2e.rs |
| MAC-05 | `sensitive = true` 字段在导出/日志中脱敏 | 正常 | macros,security | 本地文件 | src 内联 security codegen tests（`is_sensitive_effective`） | tests/e2e/macro_e2e.rs |
| MAC-06 | `flatten = true` 字段并入父命名空间 | 边界 | macros | 本地文件 | src 内联 parse/codegen tests→需集成新增 | tests/e2e/macro_e2e.rs |
| MAC-07 | `skip = true` 字段不参与加载 | 边界 | macros | 本地文件 | src 内联 codegen tests→需集成新增 | tests/e2e/macro_e2e.rs |
| MAC-08 | `dynamic = true` 字段生成 DynamicField handle | 正常 | macros,dynamic | 无 | src 内联 codegen tests + examples/dynamic_fields；集成级→需新增 | tests/e2e/macro_e2e.rs |
| MAC-09 | `merge_strategy` 六种合法值 + 非法值（`"bogus"`）宏报错 | 正常/异常 | macros | 无 | src 内联 `macros/src/parse.rs` strategy 校验 tests；trybuild→需新增 | tests/e2e/macro_e2e.rs |
| MAC-10 | `env_prefix` 结构属性：前缀剥除 + 大写下划线映射 | 正常 | macros,env | 无 | `tests/core/derive.rs::test_prefixed_config_env_mapping/test_prefixed_config_with_env_var` | tests/e2e/macro_e2e.rs |
| MAC-11 | `profile`/`profile_env` 结构属性：APP_ENV 驱动 profile overlay | 边界 | macros | 本地文件 | src 内联 parse tests（effective_profile_env）；集成级→需新增 | tests/e2e/macro_e2e.rs |
| MAC-12 | `watch = true` 结构属性：生成 watcher 挂接（与 watch feature 联动编译） | 边界 | macros,watch | 本地文件 | src 内联 codegen watch tests；集成级→需新增 | tests/e2e/macro_e2e.rs |
| MAC-13 | `version = N` 结构属性透传给 ConfigMigration（迁移版本声明） | 正常 | macros,migration | 无 | `tests/core/derive.rs::test_config_migration_derive_generates_versioned` | tests/e2e/macro_e2e.rs |
| MAC-14 | `ConfigClap` 派生：CLI 参数解析与 clap App 构建（long/short 自定义 `name_clap_long/short`） | 正常 | macros,cli | 无 | `tests/core/derive.rs::test_config_clap_derive_parses_args/test_config_clap_derive_clap_app`；examples/cli_integration | tests/e2e/macro_e2e.rs |
| MAC-15 | env 类型推断边界：bool 大小写、整数边界、浮点记数、网络值(1.2.3.4)保持字符串、JSON 数组串保持字符串 | 边界 | macros,env | 无 | `tests/core/env_types.rs`（11 例全组） | tests/e2e/macro_e2e.rs |
| MAC-16 | env 键解析不 panic：空段 `A__B`、unicode、大小写混合 | 边界 | macros,env | 无 | `tests/core/env_types.rs::test_env_parse_key_empty_segments_do_not_panic/test_env_parse_key_unicode_and_case` | tests/e2e/macro_e2e.rs |
| MAC-17 | `encrypt` 非法算法 / `env_prefix` 非法字符 → 宏展开期错误（ darling span 报错，trybuild 验收） | 异常 | macros | 无 | src 内联 parse validate tests（逻辑）；trybuild 编译级→需新增 | tests/e2e/macro_e2e.rs（trybuild） |
| MAC-18 | `interpolate = true` 字段属性 + interpolation feature：字段值模板插值后注入 | 边界 | macros,interpolation | 本地文件 | src 内联 codegen interpolate tests；集成级→需新增 | tests/e2e/macro_e2e.rs |

### 2.26 feature 组合交互（CMP，17 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| CMP-01 | encryption+watch：热更文件携带新密文，重载后用 EnvKeyProvider 新密钥解密成功；换错误密钥 → 重载失败回滚 | 正常/异常 | encryption,watch | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-02 | migration+snapshot：加载 v1 配置→迁移 v2→自动快照→load_snapshot 得 v2 内容 | 边界 | migration,snapshot | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-03 | dynamic+toggle：toggle 关闭后 dynamic 回调不再触发，开启后恢复 | 边界 | dynamic,feature-toggle | 无 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-04 | watch+config-bus：FsWatcher 检测文件变更→publish ConfigChangeEvent→订阅方执行重载 | 正常 | watch,config-bus | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-05 | watch+migration：文件回退到旧 version → MigrationOnReload::Always 自动迁移后生效 | 边界 | watch,migration | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-06 | progressive-reload+validation：健康检查含 garde 校验，坏配置 Canary 回滚 | 异常 | progressive-reload,validation | 本地文件 | `tests/core/progressive.rs::test_real_health_check_critical_on_invalid`（部分）→需文件级端到端 | tests/e2e/combo_e2e.rs |
| CMP-07 | remote+encryption：远端(HTTP 轮询)下发加密字段 → 本地密钥解密注入 | 正常 | remote,encryption | Consul(8500)（作 HTTP 源） | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-08 | etcd 轮询当热更：KV 变更→轮询感知→DynamicField 更新 | 边界 | etcd,dynamic | etcd(2379) | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-09 | audit+key rotation：KeyRing rotate 后 audit 记录 rotation 事件且旧版本仍可解密 | 正常 | audit,key | 本地文件 | `tests/security/audit.rs::test_audit_log_key_rotation`（单侧）→需组合新增 | tests/e2e/combo_e2e.rs |
| CMP-10 | security-rules+validation 双层校验：garde 结构校验 + registry 安全校验串联，报告分层输出 | 异常 | validation,security-rules | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-11 | modules+profile：ModuleRegistry 激活 profile 的文件并入主 ConfigBuilder 链 | 边界 | modules,env | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-12 | interpolation+env+file：文件模板 `${DB_HOST}` 由 env 解析，env 又被 CLI `--env-file` 注入 | 正常 | interpolation,env | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-13 | cli+schema：`ConfigClap` 参数覆盖 → validate/export 命令反映 CLI 优先值 | 边界 | cli,macros | 本地文件 | examples/cli_integration（部分）→需断言级新增 | tests/e2e/combo_e2e.rs |
| CMP-14 | full 预设全功能烟囱测试：全功能叠加一次构建+热更+加密+快照+审计全通 | 正常 | full | 本地文件 | examples/full_stack（运行验收） | tests/e2e/combo_e2e.rs |
| CMP-15 | snapshot+watch：每次重载前自动快照，连续 3 次重载后可回滚到任一历史 | 边界 | snapshot,watch | 本地文件 | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-16 | nats-bus 双实例配置同步：实例 A 热更 → NATS 广播 → 实例 B 重载一致 | 边界 | nats-bus,watch | NATS(4222) | 无→需新增 | tests/e2e/combo_e2e.rs |
| CMP-17 | context-aware+feature-toggle+dynamic：region 上下文 + 开关共同决定动态字段取值 | 边界 | context-aware,feature-toggle,dynamic | 无 | 无→需新增 | tests/e2e/combo_e2e.rs |

### 2.27 并发与竞态（CCY，8 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| CCY-01 | `InMemoryConfig` 多任务并发 set/get/delete/has 无死锁、最终一致 | 边界 | default | 无 | src 内联 `src/impl_/memory.rs` 并发 tests、`tests/core/dynamic.rs::test_real_config_extended` | tests/e2e/concurrency_e2e.rs |
| CCY-02 | watch 期间写方并发追加写（每次写完整内容）→ 每次触发重载均可完整解析（无半读） | 边界 | watch | 本地文件 | 无→需新增（对应 bench concurrent_rw_bench 的测试化） | tests/e2e/concurrency_e2e.rs |
| CCY-03 | 并发 reload + dynamic read：arc-swap 保证读侧只见到新旧两版之一 | 边界 | dynamic,watch | 本地文件 | `tests/core/progressive.rs::test_immediate_reload_atomic`（部分）→需压力化新增 | tests/e2e/concurrency_e2e.rs |
| CCY-04 | snapshot：并发 save 多份 + 同时 prune → 目录内快照数 ≤ max 且无损坏文件 | 边界 | snapshot | 本地文件 | 无→需新增 | tests/e2e/concurrency_e2e.rs |
| CCY-05 | bus：4 生产者并发 publish 1000 事件，2 订阅者合计收到全部（InMemory 容量足够时） | 边界 | config-bus | 无 | 无→需新增 | tests/e2e/concurrency_e2e.rs |
| CCY-06 | audit：8 线程并发 write 事件 → 文件可逐行 JSON 解析且行数正确 | 边界 | audit | 本地文件 | 无→需新增（=AUD-11 压力档） | tests/e2e/concurrency_e2e.rs |
| CCY-07 | toggle：高并发 toggle 与 is_enabled 读混合，无 panic、最终 list 状态自洽 | 边界 | feature-toggle | 无 | `tests/core/toggle.rs::test_concurrent_toggle_operations`（基础档）→需压力档 | tests/e2e/concurrency_e2e.rs |
| CCY-08 | 超大配置（10k 键/深度 32/10MB 文件）在 limits 内可加载，超限被拒；加载期间并发读无撕裂 | 边界 | default | 本地文件 | src 内联 limits tests（拒收侧）→需大文件集成新增 | tests/e2e/concurrency_e2e.rs |

### 2.28 feature 预设编译矩阵（PRS，8 条）

| ID | 场景描述 | 类型 | 涉及 feature | 依赖服务 | 既有覆盖 | E2E 落点 |
|----|---------|------|-------------|---------|---------|---------|
| PRS-01 | `--no-default-features`（仅 core）全库编译 + core 单测通过 | 边界 | — | 无 | CI 部分覆盖→需固化脚本 | tests/e2e/presets_e2e.rs（编译门禁） |
| PRS-02 | `--features minimal`（env+json）编译通过 | 边界 | minimal | 无 | 无→需新增 | tests/e2e/presets_e2e.rs |
| PRS-03 | `--features recommended`（toml,env,validation,json,security-rules）编译通过 | 边界 | recommended | 无 | 无→需新增 | tests/e2e/presets_e2e.rs |
| PRS-04 | `--features dev`（12 项）编译 + tests/core 通过 | 边界 | dev | 无 | 无→需新增 | tests/e2e/presets_e2e.rs |
| PRS-05 | `--features production`（15 项）编译通过 | 边界 | production | 无 | 无→需新增 | tests/e2e/presets_e2e.rs |
| PRS-06 | `--features distributed`（8 项）编译通过 | 边界 | distributed | 无 | 无→需新增 | tests/e2e/presets_e2e.rs |
| PRS-07 | `--features full`（30 项）编译 + 全部 `[[test]]` 通过（依赖 docker 服务时守卫跳过） | 边界 | full | 全部 | 现行 CI 常态→需固化为脚本断言 | tests/e2e/presets_e2e.rs |
| PRS-08 | 单 feature 逐一开启（toml/json/yaml/ini/env/validation/watch/encryption/…25 项）每项 `cargo check` 通过（依赖链自动生效：security→encryption、nats-bus→config-bus 等） | 边界 | 全部单 feature | 无 | 无→需新增（循环脚本） | tests/e2e/presets_e2e.rs |

---

## 3. feature 互斥 / 组合矩阵

### 3.1 依赖链（由 Cargo.toml `[features]` 固化，组合测试无需单独验证的部分）

```
security          → encryption, hex
security-rules    → security (→encryption), ipnet
key               → encryption, chrono, rand, hex
cli               → clap, similar, toml, json, yaml, chrono
progressive-reload→ watch, arc-swap, async-trait
snapshot          → chrono, tokio, json, toml, yaml, dynamic
etcd              → remote, etcd-client, toml, json, yaml
consul            → remote, toml, json, yaml
nats-bus          → config-bus, async-nats
redis-bus         → config-bus, redis, async-stream
typescript-schema → schema
dotenv            → env
default           → toml, json, env
```

结论：**不存在互斥 feature**——所有 feature 均可自由叠加；约束只体现为"开子项自动带上父项"。E2E 组合矩阵按 3.2 的显式组合清单验证交互行为（编译兼容由 PRS 矩阵保证）。

### 3.2 组合测试矩阵（CMP-01…17 的输入依据）

| 组合 | 组成 feature | 交互点 | 对应场景 |
|------|-------------|--------|---------|
| C1 | encryption + watch | 重载后重新解密；密钥轮换联动 | CMP-01 |
| C2 | migration + snapshot | 迁移后快照版本一致性 | CMP-02 |
| C3 | dynamic + feature-toggle | 开关门控回调 | CMP-03 |
| C4 | watch + config-bus | 文件事件→总线广播 | CMP-04 |
| C5 | watch + migration | 旧版本回退自动迁移 | CMP-05 |
| C6 | progressive-reload + validation | 健康检查门禁 | CMP-06 |
| C7 | remote + encryption | 远端密文下发 | CMP-07 |
| C8 | etcd + dynamic | 远程轮询驱动动态字段 | CMP-08 |
| C9 | audit + key | 轮换留痕 | CMP-09 |
| C10 | validation + security-rules | 双层校验 | CMP-10 |
| C11 | modules + env | profile env 切换 | CMP-11 |
| C12 | interpolation + env | 模板+env-file 注入 | CMP-12 |
| C13 | cli + macros(ConfigClap) | CLI 覆盖优先级 | CMP-13 |
| C14 | full 全家桶 | 全功能烟囱 | CMP-14 |
| C15 | snapshot + watch | 重载前快照 | CMP-15 |
| C16 | nats-bus + watch | 跨实例热更同步 | CMP-16 |
| C17 | context-aware + feature-toggle + dynamic | 三方联合取值 | CMP-17 |
| C18 | security + remote（VaultKeyProvider） | 远程密钥提供 | ENC-17 |
| C19 | snapshot 预设强依赖（json/toml/yaml/dynamic） | feature 链完整性 | PRS-07 |

### 3.3 预设覆盖关系（预设 → 展开清单，验证用 PRS-02…07）

| 预设 | 展开内容 | 备注 |
|------|---------|------|
| default | toml, json, env | 最小可用 |
| minimal | env, json | 无 toml |
| recommended | toml, env, validation, json, security-rules | security-rules 隐式带 security→encryption |
| dev | toml, json, yaml, env, cli, validation, schema, audit, watch, migration, snapshot, dynamic | snapshot 隐式带 json/toml/yaml/dynamic |
| production | toml, env, watch, encryption, validation, audit, schema, cli, migration, dynamic, progressive-reload, snapshot, security-rules, feature-toggle | progressive-reload 隐式带 watch |
| distributed | toml, json, env, watch, validation, config-bus, progressive-reload, audit | 总线仅进程内 |
| full | 30 项全量（含 etcd/consul/nats-bus/redis-bus/context-aware/modules/typescript-schema/key/interpolation…） | 需 docker 服务做行为测试 |

---

## 4. Docker 服务需求汇总

来源：`docker-compose.test.yml`（仓库根目录）。启停方式：

```bash
docker compose -f docker-compose.test.yml up -d        # 启动全部
docker compose -f docker-compose.test.yml up -d nats   # 只启动单个服务
docker compose -f docker-compose.test.yml down         # 停止并移除
```

| 服务 | 镜像 | 宿主端口 | 健康检查 | 需要它的场景域 | 无服务时的既有行为 |
|------|------|---------|---------|---------------|------------------|
| NATS | nats:2.10-alpine（`-js -m 8222`） | **4222**（监控 8222 未映射宿主） | `wget http://127.0.0.1:8222/healthz` | NAT-01…07、CMP-16；`tests/remote/bus.rs` nats 组 | `port_open("127.0.0.1",4222)` 守卫跳过 |
| Redis | redis:7 | **16379**（映射容器 6379） | `redis-cli ping` | RDS-01…07；`tests/remote/bus.rs` redis 组 | 端口守卫跳过 |
| etcd | quay.io/coreos/etcd:v3.5.16 | **2379** | `etcdctl endpoint health` | ETC-01…04/07/09/10、CMP-08、CMP-07（HTTP 轮询宿主）；`tests/remote/etcd.rs`、`tests/remote/remote.rs` source 组 | `is_service_available("http://127.0.0.1:2379/health")` 守卫跳过 |
| Consul | hashicorp/consul:1.19（dev + ui） | **8500** | `wget http://127.0.0.1:8500/v1/status/leader` | CSL-01…03/05/07/08、REM-10/11、CMP-07；`tests/remote/consul.rs` | `is_service_available` 守卫跳过（Consul dev 模式默认 token 为空/`dev-…`，E2E 固化时以 ACL 关闭态为准） |

无服务依赖域（本地文件/纯内存，可在无 docker 的 CI 沙箱跑）：FMT、BLD、VAL、IPL、ENC、KEY、SEC、AUD、WAT、PGR、DYN、TGL、MIG、SNP、MOD、CTX、SCH、CLI、MAC、CCY、PRS。
**E2E 原则**：集成/E2E 层禁 mock —— 依赖服务的场景必须打真实 compose 服务；"远程不可达"类异常场景一律使用显式不可达地址（如 `127.0.0.1:1`），不依赖网络波动。

---

## 5. 执行计划

### 5.1 层级与命令

| 层级 | 内容 | 命令 | 允许 mock？ |
|------|------|------|-----------|
| L1 单元 | src 内 61 个 `#[cfg(test)]` 模块（~1868 个内联测试）+ macros 内联测试 | `cargo test --features full --lib` | 允许 |
| L2 集成 | `tests/{core,security,remote,watcher,cli}`（需先修复 §重要发现-1 的死文件问题：把 `error` 注册回 `tests/core/mod.rs` 或迁移内容） | `cargo test --features full --test core --test security --test watcher --test cli`；`docker compose -f docker-compose.test.yml up -d && cargo test --features full --test remote` | **禁 mock**（remote 组打真实服务） |
| L3 examples | 21 个示例逐一运行（见 5.2） | `cargo run -p confers-examples --bin <name>` | 禁 mock（依赖服务的示例需先起 compose） |
| L4 E2E | `tests/e2e/`（本文档 §2 场景 ID 落点） | 新增 `[[test]]` 段（`autotests=false`）；`cargo test --features full --test e2e_*` | 禁 mock |

前置修复项（落地 E2E 前完成）：
1. `tests/core/mod.rs` 注册 `mod error;`（42 个测试复活）或将其迁移至 e2e；删除与 `tests/security/encryption.rs` 重复的 `tests/core/encryption.rs`（或二选一保留）。
2. `Cargo.toml` 增补 `[[test]]` 段：每个 `tests/e2e/*.rs` 一条，按需 `required-features`。

### 5.2 examples 运行清单（21 个，`cargo run -p confers-examples --bin <name>`，依赖 examples crate 默认 `features=["full"]`）

| # | 示例 bin | 依赖服务 | 预期结果（判验收通过） |
|---|---------|---------|----------------------|
| 1 | `basic_usage` | 无 | 退出码 0；打印加载的配置值（默认值+env 覆盖演示） |
| 2 | `hot_reload` | 本地文件 | 退出码 0；演示修改文件后新值生效（自带自改自读脚本段） |
| 3 | `remote_consul` | Consul(8500) | 退出码 0；三个 demo（basic poll / token / builder 限制）全部打印完成横幅 |
| 4 | `remote_etcd` | etcd(2379) | 退出码 0；无服务时打印启动 etcd 的提示并优雅退出（内置降级路径） |
| 5 | `encryption` | 无 | 退出码 0；加解密 roundtrip 与脱敏演示输出 |
| 6 | `key_rotation` | 无 | 退出码 0；KeyManager/轮换计划演示输出 |
| 7 | `migration` | 本地文件 | 退出码 0；v1→v2 迁移演示输出 |
| 8 | `dynamic_fields` | 无 | 退出码 0；字段更新触发回调输出 |
| 9 | `config_groups` | 本地文件 | 退出码 0；分组/profile 加载输出 |
| 10 | `progressive_reload` | 本地文件 | 退出码 0；canary/回滚演示输出 |
| 11 | `full_stack` | 无 | 退出码 0；全功能（Config+dynamic+secret+watcher）演示输出 |
| 12 | `config_bus` | 无 | 退出码 0；InMemoryBus 发布/多订阅者接收输出 |
| 13 | `snapshot` | 本地文件 | 退出码 0；快照保存/列举/恢复输出 |
| 14 | `validation` | 无 | 退出码 0；合法通过+非法被拒两分支输出 |
| 15 | `cli_integration` | 本地文件 | 退出码 0；ConfigClap 参数解析演示输出 |
| 16 | `json_schema` | 无 | 退出码 0；JSON Schema 与 TS 类型生成输出 |
| 17 | `interpolation` | 本地文件/无 | 退出码 0；`${VAR}` 与 `${VAR:default}` 插值输出 |
| 18 | `audit` | 本地文件 | 退出码 0；审计事件写入临时目录输出 |
| 19 | `context_aware` | 无 | 退出码 0；上下文切换取值输出 |
| 20 | `modules_demo` | 本地文件（部分段引用 consul/etcd 字符串仅为展示） | 退出码 0；模块注册与激活 profile 加载输出 |
| 21 | `security` | 无 | 退出码 0；安全校验/脱敏演示输出 |

批跑脚本（建议固化为 `scripts/run_examples.sh`）：`set -e; for b in $(cargo metadata --format-version 1 | jq -r '.packages[]|select(.name=="confers-examples")|.targets[].name'); do cargo run -q -p confers-examples --bin "$b"; done`

### 5.3 场景 ID → E2E 文件映射汇总

| tests/e2e/ 文件 | 覆盖场景 | 需 docker |
|----------------|---------|-----------|
| format_e2e.rs | FMT-01…22 | 否 |
| builder_e2e.rs | BLD-01…28 | 否 |
| validation_e2e.rs | VAL-01…08 | 否 |
| interpolation_e2e.rs | IPL-01…08 | 否 |
| encryption_e2e.rs | ENC-01…22 | 否 |
| key_e2e.rs | KEY-01…13 | 否 |
| security_rules_e2e.rs | SEC-01…18 | 否 |
| audit_e2e.rs | AUD-01…11 | 否 |
| watch_e2e.rs | WAT-01…18 | 否 |
| progressive_e2e.rs | PGR-01…12 | 否 |
| dynamic_e2e.rs | DYN-01…14 | 否 |
| toggle_e2e.rs | TGL-01…05 | 否 |
| migration_e2e.rs | MIG-01…10 | 否 |
| snapshot_e2e.rs | SNP-01…10 | 否 |
| modules_e2e.rs | MOD-01…11 | 否 |
| context_e2e.rs | CTX-01…08 | 否 |
| remote_e2e.rs | REM-01…11、ETC-01…10、CSL-01…08 | Consul/etcd（REM-10/11、CSL-07/08、ETC-09/10、CMP-07/08） |
| bus_e2e.rs | BUS-01…10、NAT-01…06、RDS-01…07 | NATS/Redis（NAT/RDS 组） |
| schema_e2e.rs | SCH-01…04 | 否 |
| cli_e2e.rs | CLI-01…24、SCH-05 | 否 |
| macro_e2e.rs | MAC-01…18（含 trybuild：ENC-23、MAC-17） | 否 |
| combo_e2e.rs | CMP-01…17、TGL-06、CTX-09、IPL-09 | CMP-07/08/16 需 Consul/etcd/NATS |
| concurrency_e2e.rs | CCY-01…08 | 否 |
| presets_e2e.rs | PRS-01…08、SCH-03 | 否 |

### 5.4 建议执行顺序

1. 修复死文件（5.3 前置修复项）→ 2. L1+L2 全绿（无 docker 先跑非 remote 组）→ 3. compose 起 4 服务跑 L2 remote + L3 21 示例 → 4. 按 e2e 文件逐个落地场景（优先 CMP/CCY/异常类，因既有覆盖最薄）→ 5. PRS 编译门禁进 CI（每个 PR 与 nightly 各跑一轮）。

---

## 6. 统计汇总

（按本文档场景行逐条程序化统计得出，口径见备注）

| 维度 | 数量 |
|------|------|
| 场景总数 | **357** |
| 类型=正常（纯） | 152 |
| 类型=异常（纯） | 84 |
| 类型=边界（纯） | 110 |
| 类型=正常/异常（双断言） | 8 |
| 类型=正常/边界（双断言） | 3 |
| 需新增的 E2E 场景（既有覆盖为"无→需新增"） | 44 |
| 需新增断言/集成的场景（已有单测或示例，但缺集成/行为级断言，"…→需新增"） | 36 |
| 引用 `tests/` 既有集成测试的场景（与其他口径有重叠） | 222 |
| 引用 src 内联测试的场景（与其他口径有重叠） | 91 |
| 引用 `⚠死文件` 测试的场景（落地前须迁移注册，见 §重要发现-1） | 18 |
| 引用 examples 验收的场景 | 9 |
| 依赖 docker 服务的场景（NATS/Redis/etcd/Consul 任一） | 30 |
| 仅依赖本地文件的场景 | 139 |
| 完全无外部依赖（纯内存/编译期）的场景 | 188 |

> 说明：既有覆盖口径存在合法重叠（一条场景可同时引用 tests/ 集成测试与 src 内联测试）；"需新增"取最强缺口判定。后续落地阶段允许将粒度过细的场景合并实现，但 **ID 保持稳定**以便追溯。
