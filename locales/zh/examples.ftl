# confers 示例目录(zh)。与 en/examples.ftl 键一一对应。

# --- audit ---

example-audit-title = Audit - 审计日志示例
example-audit-log-dir = 日志目录: { $path }
example-audit-build-config = [构建 AuditConfig]
example-audit-build-writer = [构建 AuditWriter]
example-audit-level-mapping = [AuditLevel 事件映射]
example-audit-event-level = 事件 { $index } -> { $level }
example-audit-write-events = [写入审计事件]
example-audit-wrote-events = 已写入 6 条审计事件
example-audit-log-contents = [审计日志内容] { $path }
example-audit-log-line = 行 { $index }: { $line }
example-audit-log-empty = (日志文件为空)
example-audit-read-log-failed = 读取日志失败: { $error }
example-audit-completed = 示例运行完成!

# --- basic_usage ---

example-basic-usage-title = Basic Usage - 基础配置加载示例
example-basic-usage-config-info = ========== 配置信息 ==========
example-basic-usage-server-section = [服务器配置]
example-basic-usage-host = 地址: { $host }
example-basic-usage-port = 端口: { $port }
example-basic-usage-workers = 工作线程: { $workers }
example-basic-usage-database-section = [数据库配置]
example-basic-usage-max-connections = 最大连接数: { $count }
example-basic-usage-feature-section = [功能配置]
example-basic-usage-logging-enabled = 启用日志: { $enabled }
example-basic-usage-metrics-enabled = 启用指标: { $enabled }
example-basic-usage-log-level = 日志级别: { $level }
example-basic-usage-app-metadata = [应用元数据]
example-basic-usage-app-name = 名称: { $name }
example-basic-usage-app-version = 版本: { $version }
example-basic-usage-port-zero = 服务器端口不能为 0
example-basic-usage-workers-zero = 工作线程数不能为 0
example-basic-usage-max-connections-zero = 最大连接数不能为 0
example-basic-usage-invalid-log-level = 无效的日志级别: { $level }（有效值: { $valid }
example-basic-usage-cwd = 当前工作目录: { $cwd }
example-basic-usage-env-section = [环境变量覆盖]
example-basic-usage-env-hint = 可以通过设置以下环境变量来覆盖配置值：
example-basic-usage-loading = 正在加载配置...
example-basic-usage-load-failed = 配置加载失败: { $error }
example-basic-usage-validating = 正在验证配置...
example-basic-usage-validation-failed = 配置验证失败: { $error }
example-basic-usage-validation-passed = ✓ 配置验证通过!
example-basic-usage-ready = 应用配置已加载并验证，可以开始运行应用...
example-basic-usage-server-address = 服务器地址: { $host }:{ $port }
example-basic-usage-logging-level = 日志功能已启用，级别: { $level }
example-basic-usage-metrics-on = 指标功能已启用
example-basic-usage-completed = 示例运行完成!

# --- config_bus ---

example-config-bus-title = ConfigBus 配置变更广播示例
example-config-bus-bus-created = ✓ InMemoryBus 已创建（容量 256）
example-config-bus-subscriber-a-subscribed = ✓ 订阅者 A 已订阅
example-config-bus-event-published-file-watcher = ✓ 事件已发布: source=file_watcher, keys=[server.port, server.host]
example-config-bus-subscriber-a-received = ✓ 订阅者 A 收到事件: instance={ $instance }, source={ $source }, checksum={ $checksum }
example-config-bus-multi-subscriber = --- 演示多订阅者广播 ---
example-config-bus-subscribers-bc-subscribed = ✓ 订阅者 B、C 已订阅（当前订阅者数: { $count }）
example-config-bus-event-published-env-override = ✓ 事件已发布: source=env_override
example-config-bus-subscriber-b-received = ✓ 订阅者 B 收到: source={ $source }, keys={ $keys }
example-config-bus-subscriber-c-received = ✓ 订阅者 C 收到: source={ $source }, keys={ $keys }
example-config-bus-create-directly = --- 演示直接创建 InMemoryBus ---
example-config-bus-received-via-in-memory-bus = ✓ 通过 InMemoryBus 收到: source={ $source }, checksum={ $checksum }
example-config-bus-completed = 示例运行完成

# --- cli_integration ---

example-cli-integration-host-description = 服务器监听地址
example-cli-integration-port-description = 服务器监听端口
example-cli-integration-workers-description = 工作线程数
example-cli-integration-log-level-description = 日志级别 (trace|debug|info|warn|error)
example-cli-integration-debug-description = 启用调试模式
example-cli-integration-override-mapping = [CLI 覆盖映射 (to_config_map)]
example-cli-integration-override-empty = (空)
example-cli-integration-title = CLI 集成示例 - ConfigClap 派生宏
example-cli-integration-how-to-run = 运行方式:
example-cli-integration-step1 = === 步骤 1: 默认配置 ===
example-cli-integration-default-config = 默认配置 (Default)
example-cli-integration-step2 = === 步骤 2: 解析 CLI 参数 (clap_args) ===
example-cli-integration-parsing-result = [CLI 解析结果]
example-cli-integration-step3 = === 步骤 3: 转换为覆盖映射 (to_config_map) ===
example-cli-integration-step4 = === 步骤 4: 合并配置 (CLI 覆盖默认值) ===
example-cli-integration-merged-config = 合并后配置
example-cli-integration-completed = 示例运行完成

# --- context_aware ---

example-context-aware-title = Context-Aware - 上下文感知配置示例
example-context-aware-value-types = [ContextValue 类型]
example-context-aware-string = 字符串: { $value } (as_str={ $as_str })
example-context-aware-number = 数字: { $value } (as_number={ $as_number })
example-context-aware-bool = 布尔: { $value } (as_bool={ $as_bool })
example-context-aware-build-upload-limit = [构建上下文感知字段 - 上传限制]
example-context-aware-free-plan = free 计划: 100MB
example-context-aware-pro-plan = pro 计划: 1GB
example-context-aware-enterprise-plan = enterprise 计划: 10GB
example-context-aware-resolve-by-plan = [按计划解析]
example-context-aware-free-bytes = free: { $bytes } 字节
example-context-aware-pro-bytes = pro: { $bytes } 字节
example-context-aware-enterprise-bytes = enterprise: { $bytes } 字节
example-context-aware-resolve-log-level = [按环境解析日志级别]
example-context-aware-resolve-endpoints = [按区域解析 API 端点]
example-context-aware-resolve-feature-flags = [按目标键解析特性开关]
example-context-aware-rule-description = [ContextRule 描述]
example-context-aware-rule-desc-text = 有 tier 属性时返回 100
example-context-aware-rule-matches = 规则匹配: { $matches }
example-context-aware-rule-value = 规则值: { $value }
example-context-aware-completed = 示例运行完成!

# --- snapshot ---

example-snapshot-title = 配置快照持久化示例
example-snapshot-in-memory-created = ✓ 内存配置已创建，写入了 server.host 和 server.port
example-snapshot-manager-created = ✓ 快照管理器已创建，输出目录: { $dir }
example-snapshot-saved = ✓ 快照已保存: { $file }
example-snapshot-redacted = ✓ 敏感字段 password 已脱敏为 [REDACTED]
example-snapshot-list-header = --- 快照列表（共 { $count } 个）---
example-snapshot-list-row = { $name } | { $size } 字节 | { $created }
example-snapshot-cleanup-demo = --- 演示快照清理（max_snapshots=5）---
example-snapshot-cleanup-removed = ✓ 手动清理删除了 { $count } 个旧快照
example-snapshot-remaining = 当前剩余快照数: { $count }
example-snapshot-completed = 示例运行完成

# --- progressive_reload ---

example-progressive-reload-title = 渐进式重载示例
example-progressive-reload-strategy-immediate = --- 策略 1: Immediate（立即重载）---
example-progressive-reload-initial-config = 初始配置: port={ $port }, host={ $host }
example-progressive-reload-current-config = 当前配置: port={ $port }, host={ $host }
example-progressive-reload-strategy-canary = --- 策略 2: Canary（金丝雀发布）---
example-progressive-reload-initial-port = 初始配置: port={ $port }
example-progressive-reload-current-port = 当前配置: port={ $port }
example-progressive-reload-strategy-linear = --- 策略 3: Linear（线性推出）---
example-progressive-reload-completed = 示例运行完成
example-progressive-reload-outcome-committed = ✓ 重载结果: Committed（已提交）
example-progressive-reload-outcome-rolled-back = ✗ 重载结果: RolledBack（已回滚）— { $reason }

# --- modules_demo ---

example-modules-demo-title = Modules - 配置模块系统示例
example-modules-demo-conf-dir = 配置目录: { $dir }
example-modules-demo-create-registry = [创建模块注册表]
example-modules-demo-registry-size = 注册表大小: { $size }
example-modules-demo-is-empty = 是否为空: { $empty }
example-modules-demo-register-groups = [注册配置组]
example-modules-demo-registered-groups = [已注册组]
example-modules-demo-group-row = 组: { $group } (活动配置: { $active })
example-modules-demo-details-database = [模块配置详情 - database]
example-modules-demo-module-name = 模块名: { $name }
example-modules-demo-active-profile = 活动配置: { $profile }
example-modules-demo-profile-count = 配置数: { $count }
example-modules-demo-available-profiles = 可用配置: { $profiles }
example-modules-demo-has-mysql = 含 mysql: { $has }
example-modules-demo-load-module = [加载模块配置]
example-modules-demo-loaded-postgresql = database/postgresql 加载成功
example-modules-demo-load-active = [加载活动配置]
example-modules-demo-loaded-redis = cache 活动配置 (redis) 加载成功
example-modules-demo-switch-active = [切换活动配置]
example-modules-demo-active-before-switch = 切换前 database 活动: { $profile }
example-modules-demo-active-after-switch = 切换后 database 活动: { $profile }
example-modules-demo-all-active = [所有活动配置]
example-modules-demo-env-parsing = [环境变量解析]
example-modules-demo-active-after-env = 环境变量解析后 database 活动: { $profile }
example-modules-demo-validate-active = [验证活动配置]
example-modules-demo-all-files-exist = 所有活动配置文件存在
example-modules-demo-validation-failed = 验证失败: { $error }
example-modules-demo-completed = 示例运行完成!

# --- schema_to_rust ---

example-schema-to-rust-title = JSON Schema → Rust 脚手架示例
example-schema-to-rust-completed = 示例运行完成
example-schema-to-rust-basic-mapping = --- 基础映射 ---
example-schema-to-rust-defs-and-refs = --- $defs 与内部 $ref ---
example-schema-to-rust-explicit-error = --- 不支持结构的显性报错 ---
example-schema-to-rust-unreachable = （不应到达）
example-schema-to-rust-failed-as-expected = 按预期失败：{ $error }

# --- interpolation ---

example-interpolation-title = Interpolation - 变量插值示例
example-interpolation-basic = [基本插值]
example-interpolation-template = 模板: { $template }
example-interpolation-result = 结果: { $result }
example-interpolation-default-values = [默认值]
example-interpolation-nested-references = [嵌套引用]
example-interpolation-nesting-in-defaults = [默认值中嵌套]
example-interpolation-traced-normal = [跟踪插值 - 普通字段]
example-interpolation-value = 值: { $value }
example-interpolation-referenced-variables = 引用变量: { $variables }
example-interpolation-is-sensitive-field = 是否敏感字段: { $sensitive }
example-interpolation-traced-sensitive = [跟踪插值 - 敏感字段]
example-interpolation-contains-sensitive-refs = 含敏感引用: { $contains }
example-interpolation-config = [插值配置]
example-interpolation-api-key-sensitive = API_KEY 是否敏感: { $sensitive }
example-interpolation-db-password-sensitive = DB_PASSWORD 是否敏感: { $sensitive }
example-interpolation-host-sensitive = HOST 是否敏感: { $sensitive }
example-interpolation-batch-context = [批量插值上下文]
example-interpolation-api-key-sensitive-ref = API_KEY 是敏感引用: { $sensitive }
example-interpolation-host-sensitive-ref = HOST 是敏感引用: { $sensitive }
example-interpolation-api-key-ref-field = API_KEY 引用自字段: { $field }
example-interpolation-warnings = [插值警告]
example-interpolation-warning = 警告: { $warning }
example-interpolation-unresolved-errors = [未解析变量错误]
example-interpolation-unreachable = 不应到达此处: { $value }
example-interpolation-expected-error = 预期错误: { $error }
example-interpolation-completed = 示例运行完成!

# --- security ---

example-security-title = Security - 安全模块示例
example-security-encryption-prefix-basics = [EncryptionPrefix 基本用法]
example-security-prefix-string = 前缀字符串: { $prefix }
example-security-value-encrypted = 值 { $value } 是否加密: { $encrypted }
example-security-strip-prefix = [去除加密前缀]
example-security-no-prefix = 无前缀
example-security-no-prefix-plaintext = { $value } 无前缀（明文）
example-security-display-fromstr = [Display 和 FromStr]
example-security-parse-failed = 解析失败
example-security-parse-failed-expected = FromStr("invalid") -> 解析失败（预期）
example-security-validator-defaults = [EnvSecurityValidator 默认]
example-security-app-port-allowed = APP_PORT 允许: { $allowed }
example-security-path-allowed = PATH 允许: { $allowed }
example-security-api-key-allowed = api_key (小写) 允许: { $allowed }
example-security-validate-names = [验证环境变量名]
example-security-name-valid = { $name } -> 有效
example-security-name-invalid = { $name } -> 无效: { $error }
example-security-validate-values = [验证环境变量值]
example-security-value-valid = { $value } -> 有效
example-security-value-invalid = { $value } -> 无效: { $error }
example-security-encrypted-bypass = [加密值绕过验证]
example-security-encrypted-value-passes = 加密值通过值验证
example-security-encrypted-value-rejected = 加密值被拒绝: { $error }
example-security-api-key-passes = API_KEY（加密值）通过名验证
example-security-api-key-rejected = API_KEY 被拒绝: { $error }
example-security-custom-config = [自定义验证配置]
example-security-name-64-valid = 64 字符名有效: { $valid }
example-security-name-65-valid = 65 字符名有效: { $valid }
example-security-strict-lenient = [严格与宽松验证器]
example-security-log-redaction = [日志脱敏]
example-security-original-length = 原始长度: { $length }
example-security-redacted-length = 脱敏长度: { $length } (以 ... 结尾)
example-security-completed = 示例运行完成!

# --- remote_consul ---

example-remote-consul-title = Remote Consul 配置源示例
example-remote-consul-demo1 = === 演示 1: 基础 Consul 配置轮询 ===
example-remote-consul-built = ConsulSource 构建成功
example-remote-consul-source-id = 源 ID: { $id }
example-remote-consul-poll-interval = 轮询间隔: { $interval }
example-remote-consul-polling = 正在轮询 Consul KV (prefix: myapp/config)...
example-remote-consul-loaded = ✓ 配置加载成功
example-remote-consul-value-map = 值类型: Map (包含多个键值对)
example-remote-consul-value-null = 值类型: Null (前缀下无配置)
example-remote-consul-value-scalar = 值类型: 标量值
example-remote-consul-poll-failed-normal = ✗ 轮询失败（如果没有运行 Consul 这是正常的）
example-remote-consul-error = 错误: { $error }
example-remote-consul-hint = 提示: 运行以下命令启动 Consul:
example-remote-consul-demo2 = === 演示 2: 带 ACL Token 的连接 ===
example-remote-consul-with-token-built = 带 Token 的 ConsulSource 构建成功
example-remote-consul-loaded-token-verified = ✓ 配置加载成功（Token 验证通过）
example-remote-consul-poll-failed = ✗ 轮询失败: { $error }
example-remote-consul-token-note = 注意: 生产环境应始终使用 HTTPS + ACL Token
example-remote-consul-demo3 = === 演示 3: 构建器参数与安全限制 ===
example-remote-consul-security-limits = ConsulSource 安全限制配置:
example-remote-consul-max-response-body = 最大响应体: 4 MB
example-remote-consul-max-kv-entries = 最大 KV 条目数: 5000
example-remote-consul-dos-limits-note = 这些限制防止超大响应导致的 DoS/OOM (CWE-400/CWE-502)
example-remote-consul-address = Consul 地址: { $address }
example-remote-consul-completed = 示例运行完成

# --- remote_etcd ---

example-remote-etcd-title = Remote etcd 配置源示例
example-remote-etcd-demo1 = === 演示 1: 基础 etcd 配置轮询 ===
example-remote-etcd-built = EtcdSource 构建成功
example-remote-etcd-source-id = 源 ID: { $id }
example-remote-etcd-poll-interval = 轮询间隔: { $interval }
example-remote-etcd-build-failed-normal = ✗ EtcdSource 构建失败（如果没有运行 etcd 这是正常的）
example-remote-etcd-error = 错误: { $error }
example-remote-etcd-hint = 提示: 运行以下命令启动 etcd:
example-remote-etcd-polling = 正在轮询 etcd KV (prefix: myapp/config)...
example-remote-etcd-loaded = ✓ 配置加载成功
example-remote-etcd-value-map = 值类型: Map (包含多个键值对)
example-remote-etcd-value-null = 值类型: Null (前缀下无配置)
example-remote-etcd-value-scalar = 值类型: 标量值
example-remote-etcd-poll-failed-normal = ✗ 轮询失败（如果没有运行 etcd 这是正常的）
example-remote-etcd-demo2 = === 演示 2: 带认证的 etcd 连接 ===
example-remote-etcd-with-auth-built = 带认证的 EtcdSource 构建成功
example-remote-etcd-poll-failed = ✗ 轮询失败: { $error }
example-remote-etcd-build-failed = ✗ 构建失败: { $error }
example-remote-etcd-credentials-note = 注意: 生产环境应从环境变量读取认证凭证，避免硬编码
example-remote-etcd-demo3 = === 演示 3: TLS 配置与多端点集群 ===
example-remote-etcd-with-tls-built = 带 TLS 的 EtcdSource 构建成功
example-remote-etcd-build-failed-unreachable = ✗ 构建失败（远程端点不可达是正常的）
example-remote-etcd-tls-steps = 生产环境 TLS 配置步骤:
example-remote-etcd-tls-step-1 = 1. 生成 CA 证书和密钥
example-remote-etcd-tls-step-2 = 2. 为 etcd 服务器生成证书
example-remote-etcd-tls-step-3 = 3. 为客户端生成证书
example-remote-etcd-tls-step-4 = 4. etcd 启动时启用 TLS (--cert-file, --key-file, --client-cert-auth)
example-remote-etcd-tls-step-5 = 5. 客户端通过 TLS 端点连接
example-remote-etcd-endpoint = etcd 端点: { $endpoint }
example-remote-etcd-completed = 示例运行完成

# --- migration ---

example-migration-title = === 配置版本迁移示例 ===
example-migration-version-chain = 版本: V1 -> V2 -> V3
example-migration-all-completed = === 所有示例完成 ===
example-migration-basic-demo = --- 基础迁移示例 ---
example-migration-execute = 执行迁移: v{ $from } -> v{ $to }
example-migration-succeeded = 迁移成功! 版本: { $from } -> { $to }
example-migration-version-field = 迁移后 version 字段: { $version }
example-migration-failed = 迁移失败: { $error }
example-migration-path-demo = --- 路径预计算示例 ---
example-migration-migrate-1-2 = 迁移 1 -> 2
example-migration-migrate-2-3 = 迁移 2 -> 3
example-migration-migrate-1-3-direct = 直接迁移 1 -> 3
example-migration-path-1-3 = 迁移路径 (1 -> 3): { $path }
example-migration-path-1-2 = 迁移路径 (1 -> 2): { $path }
example-migration-path-2-3 = 迁移路径 (2 -> 3): { $path }
example-migration-optimal-path = 找到最优路径: { $path }
example-migration-multi-step-demo = --- 多步迁移示例 ---
example-migration-step-1 = [Step 1] 添加 database 配置
example-migration-step-2 = [Step 2] 添加 cache 和 security 配置
example-migration-initial-version = 初始配置 version: { $version }
example-migration-multi-step-succeeded = 多步迁移成功!
example-migration-final-version = 最终版本: { $version }
example-migration-config-keys = 配置键: { $keys }
example-migration-on-reload-demo = --- MigrationOnReload 示例 ---
example-migration-always-desc = MigrationOnReload::Always: 每次重载都执行迁移
example-migration-on-version-change-desc = MigrationOnReload::OnVersionChange: 仅版本变化时迁移 (默认)
example-migration-disabled-desc = MigrationOnReload::Disabled: 禁用重载时迁移
example-migration-current-always = 当前配置: Always
example-migration-current-on-version-change = 当前配置: OnVersionChange (推荐)
example-migration-current-disabled = 当前配置: Disabled
example-migration-hot-reload-usage = 在热重载场景中使用:
example-migration-auto-migrate-comment = // 配置变化时自动迁移

# --- dynamic_fields ---

example-dynamic-fields-start = === DynamicField 示例启动 ===
example-dynamic-fields-all-completed = === 所有示例运行完成 ===
example-dynamic-fields-basic-demo = --- 1. 基础用法演示 ---
example-dynamic-fields-initial-value = 创建 DynamicField，初始值: { $value }
example-dynamic-fields-lock-free-read = 无锁读取: { $value }
example-dynamic-fields-after-update = 更新后值: { $value }
example-dynamic-fields-callback-guard-demo = --- 2. CallbackGuard 生命周期管理 ---
example-dynamic-fields-initial-payload = 初始值
example-dynamic-fields-callbacks-before = 注册回调前回调数量: { $count }
example-dynamic-fields-callback-fired = [回调触发] 值变化为: { $value }
example-dynamic-fields-callbacks-after = 注册回调后回调数量: { $count }
example-dynamic-fields-first-update = 第一次更新
example-dynamic-fields-second-update = 第二次更新
example-dynamic-fields-callback-times = 回调触发次数: { $count }
example-dynamic-fields-callbacks-after-drop = guard 销毁后回调数量: { $count }
example-dynamic-fields-builder-demo = --- 3. Builder 模式 ---
example-dynamic-fields-builder-value = 使用 Builder 创建的字段值: { $value }
example-dynamic-fields-default-value = Default 创建的字段值: { $value }
example-dynamic-fields-perf-demo = --- 4. 性能对比演示 ---
example-dynamic-fields-lock-free-time = 无锁读取 (DynamicField) { $iterations } 次耗时: { $duration }
example-dynamic-fields-rwlock-time = RwLock 读取 { $iterations } 次耗时: { $duration }
example-dynamic-fields-speedup = 无锁读取比 RwLock 快 { $speedup } 倍
example-dynamic-fields-multi-callback-demo = --- 5. 多个回调同时监听 ---
example-dynamic-fields-current-callbacks = 当前回调数量: { $count }
example-dynamic-fields-after-update-10 = 更新值为 10 后:
example-dynamic-fields-callback-a-value = 回调 A 累计值: { $value }
example-dynamic-fields-callback-b-value = 回调 B 累计值: { $value }
example-dynamic-fields-callback-c-value = 回调 C 累计值: { $value }
example-dynamic-fields-callbacks-after-guard-b = 销毁 guard_b 后回调数量: { $count }
example-dynamic-fields-after-update-5 = 更新值为 5 后:

# --- hot_reload ---

example-hot-reload-started = === 热重载示例程序启动 ===
example-hot-reload-watched-file = 被监听的配置文件: { $path }
example-hot-reload-watcher-settings = WatcherConfig 配置: debounce={ $debounce }ms, min_reload_interval={ $min_interval }ms, max_failures={ $max_failures }
example-hot-reload-port-not-found = 模板中未找到 "{ $port }", 无法生成演示修改
example-hot-reload-demo-write-completed = 演示写入完成: 已把配置文件中的 { $old } 修改为 { $new }
example-hot-reload-demo-write-failed = 演示写入失败: { $error }
example-hot-reload-waiting = 等待配置文件变化... (示例将在 { $seconds }s 后自动修改配置文件)
example-hot-reload-ctrl-c-hint = 按 Ctrl+C 可提前退出程序
example-hot-reload-exit-signal = 收到退出信号，正在关闭...
example-hot-reload-reload-skipped = 跳过重载: 距离上次重载仅 { $elapsed }ms (最小间隔: { $min_interval }ms)
example-hot-reload-change-detected = 检测到配置文件变化: { $path }
example-hot-reload-reload-succeeded = 配置重载成功!
example-hot-reload-reload-failed = 配置重载失败: { $error } (连续失败: { $failures }/{ $max_failures })
example-hot-reload-failure-limit = 连续失败次数达到上限，暂停监听 { $pause }ms
example-hot-reload-watcher-stopped = 文件监听已停止或失败, 未能完成热重载演示
example-hot-reload-timeout = 在 { $seconds }s 内未收到配置变更事件, 热重载演示未完成。
example-hot-reload-no-fs-events = 当前环境可能不支持文件系统事件通知 (网络文件系统、部分容器挂载等)。
example-hot-reload-demo-completed = === 热重载演示完成，程序退出 ===
example-hot-reload-demo-not-completed = 热重载演示未完成
example-hot-reload-exiting = === 程序退出 ===
example-hot-reload-current-config = 当前配置 (已重载)
example-hot-reload-app-line = 应用: { $name } v{ $version } (环境: { $environment })
example-hot-reload-server-line = 服务器: { $host }:{ $port } (最大连接数: { $max_connections })
example-hot-reload-logs-line = 日志: 级别={ $level }, 格式={ $format }, 输出={ $output }
example-hot-reload-database-line = 数据库: { $url } (最大连接: { $max_connections }, 超时: { $timeout }s)
example-hot-reload-cache-line = 缓存: 启用={ $enabled }, TTL={ $ttl }s, 最大={ $max }MB

# --- config_groups ---

example-config-groups-started = 配置组示例启动
example-config-groups-title = 配置组（Config Groups）示例
example-config-groups-current-env = 当前环境: { $env }
example-config-groups-builder-header = ========== 使用 ConfigBuilder 加载配置 ==========
example-config-groups-chain-header = ========== 使用 SourceChainBuilder 加载配置 ==========
example-config-groups-environment = 环境: { $env }
example-config-groups-source-chain = 配置源链:
example-config-groups-merged-sections = 合并后的顶层配置段: { $sections }
example-config-groups-priority-header = ========== 配置优先级演示 ==========
example-config-groups-priority-result = 优先级演示结果:
example-config-groups-priority-name = name = { $name } (高优先级覆盖低优先级)
example-config-groups-priority-port = port = { $port } (高优先级覆盖低优先级)
example-config-groups-priority-level = level = { $level } (高优先级未设置，保留低优先级)
example-config-groups-final-header = ========== 最终配置 ==========
example-config-groups-app-section = [应用元数据]
example-config-groups-app-name = 名称: { $name }
example-config-groups-app-version = 版本: { $version }
example-config-groups-app-environment = 环境: { $environment }
example-config-groups-server-section = [服务器配置]
example-config-groups-server-host = 地址: { $host }
example-config-groups-server-port = 端口: { $port }
example-config-groups-server-workers = 工作线程: { $workers }
example-config-groups-database-section = [数据库配置]
example-config-groups-database-max-connections = 最大连接数: { $count }
example-config-groups-database-timeout = 连接超时: { $seconds }s
example-config-groups-logging-section = [日志配置]
example-config-groups-logging-enabled = 启用: { $enabled }
example-config-groups-logging-level = 级别: { $level }
example-config-groups-logging-format = 格式: { $format }
example-config-groups-cache-section = [缓存配置]
example-config-groups-cache-enabled = 启用: { $enabled }
example-config-groups-security-section = [安全配置]
example-config-groups-security-origins = 允许的源: { $origins }
example-config-groups-dev-section = [开发环境配置]
example-config-groups-dev-hot-reload = 热重载: { $enabled }
example-config-groups-dev-debug-endpoints = 调试端点: { $enabled }
example-config-groups-dev-verbose-errors = 详细错误: { $enabled }
example-config-groups-prod-section = [生产环境配置]
example-config-groups-prod-health-check = 健康检查: { $enabled }
example-config-groups-prod-rate-limit = 限流: { $enabled }
example-config-groups-prod-max-request-size = 最大请求大小: { $bytes } bytes
example-config-groups-prod-monitoring = 性能监控: { $enabled }
example-config-groups-builder-loaded = ✓ ConfigBuilder 加载成功!
example-config-groups-builder-failed = ✗ ConfigBuilder 加载失败: { $error }
example-config-groups-chain-loaded = ✓ SourceChainBuilder 加载成功!
example-config-groups-chain-failed = ✗ SourceChainBuilder 加载失败: { $error }
example-config-groups-comparison-header = ========== 环境配置对比 ==========
example-config-groups-dev-traits = dev 环境特点: localhost, port=3000, debug 日志, 关闭缓存
example-config-groups-prod-traits = prod 环境特点: 0.0.0.0, port=8080, warn 日志, 开启缓存
example-config-groups-override-header = ========== 环境变量覆盖演示 ==========
example-config-groups-override-hint = 你可以使用以下环境变量覆盖配置:
example-config-groups-override-port = APP_SERVER_PORT=9000      - 覆盖服务器端口
example-config-groups-override-database-url = APP_DATABASE_URL=...       - 覆盖数据库 URL
example-config-groups-override-log-level = APP_LOGGING_LEVEL=debug   - 覆盖日志级别
example-config-groups-override-example = 示例: APP_SERVER_PORT=9000 cargo run -p confers-examples --bin config_groups -- dev
example-config-groups-completed = 示例运行完成!

# --- key_rotation ---

example-key-rotation-started = 密钥轮换示例程序启动
example-key-rotation-all-completed = 所有示例演示完成
example-key-rotation-example-1 = === 示例 1: 密钥版本派生 ===
example-key-rotation-db-password-v1-key = 数据库密码 v1 密钥: { $key }
example-key-rotation-db-password-v2-key = 数据库密码 v2 密钥: { $key }
example-key-rotation-different-versions = 不同版本产生不同密钥: 验证通过
example-key-rotation-different-fields = 不同字段产生不同密钥: 验证通过
example-key-rotation-deterministic = 相同版本产生相同密钥（确定性）: 验证通过
example-key-rotation-example-2 = === 示例 2: 使用版本化密钥加密 ===
example-key-rotation-v1-nonce = v1 加密 - Nonce: { $nonce }
example-key-rotation-v1-ciphertext = v1 加密 - 密文: { $ciphertext }
example-key-rotation-v2-nonce = v2 加密 - Nonce: { $nonce }
example-key-rotation-v2-ciphertext = v2 加密 - 密文: { $ciphertext }
example-key-rotation-different-ciphertexts = 不同版本密钥产生不同密文: 验证通过
example-key-rotation-decrypt-correctly = 各自版本密钥正确解密: 验证通过
example-key-rotation-cross-version-fail = 跨版本密钥无法解密: 验证通过
example-key-rotation-example-3 = === 示例 3: 密钥轮换流程 ===
example-key-rotation-old-version = 旧版本: { $version }
example-key-rotation-old-nonce = 旧 Nonce: { $nonce }
example-key-rotation-old-ciphertext = 旧密文: { $ciphertext }
example-key-rotation-old-decrypted = 旧密钥解密成功: { $value }
example-key-rotation-new-version = 新版本: { $version }
example-key-rotation-new-nonce = 新 Nonce: { $nonce }
example-key-rotation-new-ciphertext = 新密文: { $ciphertext }
example-key-rotation-rotated-data = 轮换后的加密数据: { $data }
example-key-rotation-rotation-completed = 密钥轮换完成
example-key-rotation-example-4 = === 示例 4: 加密数据迁移 ===
example-key-rotation-stored-old-data = 存储的旧数据: version={ $version }, nonce 长度={ $nonce_length }, ciphertext 长度={ $ciphertext_length }
example-key-rotation-migrated-data = 迁移后的数据: version={ $version }, nonce 长度={ $nonce_length }, ciphertext 长度={ $ciphertext_length }
example-key-rotation-integrity-passed = 数据完整性验证: 通过
example-key-rotation-old-key-cannot-decrypt = 旧密钥无法解密新数据: 验证通过
example-key-rotation-example-5 = === 示例 5: EnvKeyProvider 与密钥版本 ===
example-key-rotation-master-key-ok = 获取主密钥成功
example-key-rotation-key-length = 密钥长度: { $bytes } 字节
example-key-rotation-version-keys = 版本 { $version } - Nonce: { $nonce }, Ciphertext: { $ciphertext }
example-key-rotation-env-provider-ok = EnvKeyProvider 与多版本密钥集成: 验证通过
example-key-rotation-get-key-failed = 获取密钥失败: { $error }
example-key-rotation-example-6 = === 示例 6: 回滚机制 ===
example-key-rotation-initial-v1 = 初始状态: 数据使用 v1 加密
example-key-rotation-after-rotation-v2 = 轮换后: 数据使用 v2 加密
example-key-rotation-stored-encrypted-data = 存储的加密数据: version={ $version }, previous_version={ $previous_version }
example-key-rotation-current-version-ok = ✓ 当前版本 (v2) 解密成功
example-key-rotation-historical-ok = ✓ 使用 previous_version 解密历史数据成功
example-key-rotation-rollback-complete = 回滚完成: 数据版本重置为 v1
example-key-rotation-current-previous = 当前版本: { $version }, previous_version: { $previous_version }
example-key-rotation-rollback-verified = ✓ 回滚后数据解密验证通过
example-key-rotation-security-verified = ✓ v1 密钥无法解密 v2 加密的数据（安全性验证通过）

# --- json_schema ---

example-json-schema-desc-application-name = 应用名称
example-json-schema-desc-server-port = 服务器端口
example-json-schema-desc-enable-debug-mode = 启用调试模式
example-json-schema-desc-full-app-config = 完整应用配置，包含服务器、数据库和缓存配置
example-json-schema-desc-application-version = 应用版本
example-json-schema-desc-host-address = 主机地址
example-json-schema-desc-connection-timeout = 连接超时时间（秒）
example-json-schema-desc-database-host = 数据库主机
example-json-schema-desc-database-port = 数据库端口
example-json-schema-desc-database-name = 数据库名称
example-json-schema-desc-max-connections = 最大连接数
example-json-schema-desc-caching-enabled = 是否启用缓存
example-json-schema-desc-cache-ttl = 缓存 TTL（秒）
example-json-schema-desc-cache-max-entries = 缓存最大条目数
example-json-schema-desc-hostname = 主机名（符合 RFC 1123）
example-json-schema-desc-port-number = 端口号
example-json-schema-desc-admin-email = 管理员邮箱
example-json-schema-desc-service-url = 服务 URL
example-json-schema-desc-rate-limit = 每秒请求限制
example-json-schema-desc-cache-ttl-24h = 缓存 TTL（秒，0-24h）
example-json-schema-desc-config-tags = 配置标签
example-json-schema-demo1 = === 演示 1: 简单配置 Schema 生成 ===
example-json-schema-generated-schema = 生成的 JSON Schema:
example-json-schema-validate-valid = 验证有效配置: { $status }
example-json-schema-validate-invalid = 验证无效配置: { $status }
example-json-schema-demo2 = === 演示 2: 嵌套配置 Schema ===
example-json-schema-nested-schema = 生成的嵌套 JSON Schema (definitions 部分):
example-json-schema-validate-full = 验证完整配置: { $status }
example-json-schema-demo3 = === 演示 3: 带验证约束的 Schema ===
example-json-schema-validated-schema = 带验证约束的 Schema:
example-json-schema-case-valid-config = 有效配置
example-json-schema-case-invalid-port = 无效端口
example-json-schema-case-missing-required = 缺少必需字段
example-json-schema-demo4 = === 演示 4: TypeScript 类型生成 ===
example-json-schema-generated-typescript = 生成的 TypeScript 类型定义:
example-json-schema-ide-integration = IDE 集成:
example-json-schema-ide-copy-types = - 将 TypeScript 类型定义文件复制到前端项目
example-json-schema-ide-quicktype = - 使用 JSON Schema 生成器（如 quicktype）生成更多类型
example-json-schema-ide-vscode = - 在 VS Code 中使用 schema 文件获取智能提示
example-json-schema-used-in-typescript = // 在 TypeScript 中使用:
example-json-schema-demo5 = === 演示 5: Schema 导出到文件 ===
example-json-schema-schema-exported = Schema 已导出到: { $path }
example-json-schema-verify-import-ok = 验证导入: 成功
example-json-schema-schema-title = Schema 标题: { $title }
example-json-schema-verify-import-failed = 验证导入: 失败 - { $error }
example-json-schema-export-failed = 导出失败: { $error }
example-json-schema-typescript-export-failed = TypeScript 导出失败: { $error }
example-json-schema-typescript-exported = TypeScript 类型已导出到: { $path }
example-json-schema-demo6 = === 演示 6: ConfigSchema 派生宏 ===
example-json-schema-derive-intro = 使用 #[derive(ConfigSchema)] 自动生成 Schema:
example-json-schema-auto-generated-methods = // 自动生成方法:
example-json-schema-export-schema-comment = // 导出 schema:
example-json-schema-demo7 = === 演示 7: Schema 应用场景 ===
example-json-schema-use-case-1 = 1. IDE 智能提示
example-json-schema-use-case-1a = - 将 schema 文件添加到 .vscode/settings.json
example-json-schema-use-case-1b = - JSON/YAML 配置文件获得自动完成
example-json-schema-use-case-2 = 2. 配置验证
example-json-schema-use-case-2a = - 启动时验证配置文件符合 schema
example-json-schema-use-case-2b = - CI/CD 阶段验证配置变更
example-json-schema-use-case-3 = 3. API 文档
example-json-schema-use-case-3a = - 自动生成 OpenAPI/Swagger 规范
example-json-schema-use-case-3b = - 保持前端和后端类型同步
example-json-schema-use-case-4 = 4. 多语言代码生成
example-json-schema-use-case-4a = - 使用 quicktype 从 schema 生成 Go/TypeScript/Python 类型
example-json-schema-use-case-4b = - 保持各语言类型定义一致
example-json-schema-use-case-5 = 5. 配置编辑器
example-json-schema-use-case-5a = - 在 Web UI 中渲染配置表单
example-json-schema-use-case-5b = - 从 schema 生成验证规则
example-json-schema-cli-exported = Schema 已导出到: { $path }
example-json-schema-completed = 示例运行完成

# --- validation ---

example-validation-title = Validation Example - 配置验证
example-validation-passed = 验证通过
example-validation-failed-count = 验证失败 ({ $count } 个错误):
example-validation-pretty-ok = [OK] 配置验证通过
example-validation-pretty-fail = [FAIL] 配置验证失败
example-validation-error-field = - 字段: { $field }
example-validation-error-reason = 原因: { $reason }
example-validation-total-errors = 共 { $count } 个验证错误
example-validation-host-empty = 主机地址不能为空
example-validation-host-too-long = 主机地址超过最大长度 253 字符 (当前: { $length })
example-validation-host-invalid-chars = 主机地址包含无效字符: { $host }
example-validation-port-reserved = 端口不能为 0（保留端口）
example-validation-port-privileged = 端口 { $port } 是特权端口（< 1024），建议使用非特权端口
example-validation-log-level-invalid = 无效的日志级别: '{ $level }' (有效值: { $valid })
example-validation-email-empty = 邮箱不能为空
example-validation-email-missing-at = 邮箱格式无效：缺少 @ 符号
example-validation-email-multiple-at = 邮箱格式无效：包含多个 @ 符号
example-validation-email-local-empty = 邮箱格式无效：用户名部分为空
example-validation-email-domain-empty = 邮箱格式无效：域名部分为空
example-validation-email-domain-dot = 邮箱格式无效：域名必须包含 .
example-validation-timeout-positive = 超时时间必须大于 0
example-validation-timeout-too-long = 超时时间过长: { $timeout }s (最大: 86400s = 24h)
example-validation-timeout-too-short = 超时时间过短: { $timeout }s (最小: 1s)
example-validation-max-connections-zero = 最大连接数不能为 0
example-validation-max-connections-too-many = 最大连接数过多: { $connections } (建议: <= 10000)
example-validation-url-empty = URL 不能为空
example-validation-url-scheme = URL 必须以有效协议开头 ({ $schemes })
example-validation-demo1 = === 演示 1: 基础字段验证 ===
example-validation-host-header = 主机地址验证:
example-validation-item-error = 错误: { $error }
example-validation-port-header = 端口范围验证:
example-validation-item-warning = 警告: { $error }
example-validation-log-level-header = 日志级别验证:
example-validation-demo2 = === 演示 2: 格式验证 ===
example-validation-email-header = 邮箱格式验证:
example-validation-db-url-header = 数据库 URL 验证:
example-validation-demo3 = === 演示 3: 验证上下文 ===
example-validation-default-context = 默认上下文验证结果:
example-validation-after-ignore = 忽略 'port' 和 'timeout_seconds' 后:
example-validation-strict-mode = 严格模式（显示详细信息）:
example-validation-demo4 = === 演示 4: 有效配置验证 ===
example-validation-validate-default = 验证默认配置:
example-validation-result-header = 验证结果:
example-validation-demo5 = === 演示 5: 验证规则解析 ===
example-validation-unknown-rule = '{ $rule }' -> (未知规则)
example-validation-demo6 = === 演示 6: 错误处理策略 ===
example-validation-strategy-1 = 策略 1: 如果验证失败则退出程序
example-validation-detected-errors = 检测到 { $count } 个错误，应用无法启动
example-validation-fix-and-restart = 请修复配置后重新启动
example-validation-strategy-2 = 策略 2: 使用哨兵值允许带警告运行
example-validation-demo7 = === 演示 7: 与 Config Derive 集成 ===
example-validation-derive-intro = ConfigDerive 验证集成方式:
example-validation-method-1 = 方式 1: 使用 #[config(validate)] 自动验证
example-validation-method-2 = 方式 2: 手动调用 validate() 方法
example-validation-validation-failed-print = eprintln!("验证失败: {"{"}:?{"}"}", report);
example-validation-method-3 = 方式 3: 使用 ValidationContext 自定义验证
example-validation-completed = 示例运行完成
