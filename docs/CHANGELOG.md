# 更新日志

本文件记录本项目的全部重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，
版本号遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

## [Unreleased]

_暂无变更。_

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
