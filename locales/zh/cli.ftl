# confers CLI 目录(zh)。与 en/cli.ftl 键一一对应。

cli-about = confers 配置诊断工具
cli-lang-help = UI 语言,使用 BCP-47 标签(`zh`/`en`; 其他格式正确的标签回退到英文文案)。覆盖 CONFERS_LANG 与语言环境检测
cli-error-prefix = 错误
cli-inspect-title = 配置巡检
cli-inspect-loaded-sources = 已加载 { $count } 个配置源
cli-inspect-all-keys = 全部配置键:
cli-inspect-requested-keys = 指定的配置键:
cli-col-key = 键
cli-col-value = 值
cli-col-source = 来源
cli-col-location = 位置
cli-location-line-col = 行 { $line },列 { $col }
cli-not-found = [未找到]
cli-reveal-warning = 警告: 已启用 --reveal: 敏感值将原样打印
cli-snapshot-none-found = { $directory } 中未找到快照
cli-snapshot-file-missing = 快照文件不存在: { $file }
cli-snapshot-runtime-build-failed = 运行时构建失败: { $message }
cli-snapshot-restored = 已恢复: { $file }
cli-snapshot-top-level-keys = 顶层配置键数: { $count }
cli-doctor-encryption-not-compiled = 编译时未启用 encryption 特性; 加载时加密值将无法解密
cli-schema-absolute-path-not-allowed = 不允许使用绝对路径: { $path }; 可通过 --allow-absolute-paths 覆盖此限制
cli-schema-read-failed = 读取 schema 失败: { $path }
cli-schema-parse-failed = 解析 schema 失败: { $path }
cli-docs-global-args-lang = --lang <bcp47> (zh/en; 其他标签回退到 en)

# --- clap 帮助/说明 (Cli, Commands, SnapshotCommands) ---

cli-config-help = 要加载的配置文件(可重复)
cli-env-file-help = 附加的环境变量文件
cli-allow-absolute-paths-help = 允许配置文件使用绝对路径(请谨慎使用,主要用于测试)
cli-fields-help = 只输出指定字段(逗号分隔的点路径,例如 "a.b,c.d")
cli-inspect-help = 巡检配置 - 列出所有配置键及其来源
cli-inspect-key-help = 只显示指定的配置键
cli-inspect-show-conflicts-help = 显示所有冲突(被覆盖的值)
cli-inspect-format-help = 输出格式 (text, json)
cli-inspect-reveal-help = 显示敏感字段的值(默认脱敏)
cli-validate-help = 按 schema 校验配置
cli-validate-strict-help = 严格模式: 把警告视为错误
cli-validate-format-help = 输出格式 (text, json)
cli-export-help = 导出合并后的配置(默认脱敏)
cli-export-format-help = 输出格式 (json, toml, yaml)
cli-export-output-help = 输出文件(默认输出到标准输出)
cli-export-with-provenance-help = 包含来源(provenance)信息
cli-export-raw-help = 不对敏感值脱敏
cli-diff-help = 比对两份配置
cli-diff-base-help = 基准配置文件
cli-diff-overlay-help = 叠加配置文件
cli-diff-format-help = 输出格式 (text, json)
cli-diff-sanitize-help = 对输出中的敏感值脱敏
cli-snapshot-help = 管理配置快照
cli-snapshot-list-help = 列出全部快照
cli-snapshot-list-directory-help = 存放快照的目录
cli-snapshot-diff-help = 比对两份快照
cli-snapshot-diff-latest-help = 参与比对的最近快照数量
cli-snapshot-diff-directory-help = 存放快照的目录
cli-snapshot-prune-help = 清理旧快照
cli-snapshot-prune-older-than-help = 保留比该时间更新的快照(例如 "30d"、"7d")
cli-snapshot-prune-directory-help = 存放快照的目录
cli-snapshot-restore-help = 恢复(加载并校验)快照文件(灾难恢复)
cli-snapshot-restore-file-help = 要恢复的快照文件; 省略时取目录中最新的快照
cli-snapshot-restore-directory-help = 存放快照的目录
cli-schema-help = 输出配置类型的 JSON Schema
cli-schema-from-instance-help = 从已加载的配置实例推断 schema 草稿,而不是使用派生类型(schema-first 逆向工程)
cli-schema-from-schema-help = 从 JSON Schema 草稿文件生成 Rust 结构体脚手架(2020-12 风格; 非必填字段变为 Option,标量默认值生成 default 函数,字符串枚举生成 Rust 枚举; 不支持的构造会直接报错)
cli-get-help = 按键路径(点分隔)获取单个配置值
cli-get-key-help = 配置键路径(例如 "server.host" 或 "database.pool.size")
cli-get-reveal-help = 显示敏感字段的值(默认脱敏)
cli-docs-help = 文档输出。`--agent` 输出机器可读知识包(子命令、参数、退出码契约、任务配方)
cli-docs-agent-help = 输出 agent 知识包
cli-docs-format-help = 知识包格式 (json, markdown)
cli-doctor-help = 诊断配置健康度(schema、来源链、加密、环境变量覆盖)并输出单行 JSON 报告
cli-doctor-format-help = 输出格式 (json = 单行, text)

# --- 环境变量文件加载错误 ---

cli-env-file-not-found = 环境变量文件不存在: { $path }
cli-env-file-stat-failed = 读取环境变量文件属性失败: { $path }
cli-env-file-too-large = 环境变量文件过大: 超出大小上限 { $limit } 字节(文件为 { $size } 字节): { $path }
cli-env-file-read-failed = 读取环境变量文件失败: { $path }
cli-env-file-line-too-long = 环境变量文件单行过长: 超出上限 { $limit } 字节(该行为 { $size } 字节): { $path }

# --- validate ---

cli-unsupported-format = 不支持的格式: { $format }
cli-validate-title = 配置校验
cli-validate-loaded-ok = ✓ 配置加载成功
cli-validate-issues-found = ✗ 发现 { $count } 个校验问题:
cli-validate-all-passed = ✓ 全部校验项通过
cli-validate-strict-failed = 校验失败: 共 { $count } 个问题
cli-validate-error = ✗ 配置错误: { $message }
cli-validate-failed = 校验失败

# --- export --raw ---

cli-export-raw-warning = ⚠️  警告: --raw 选项将导出未脱敏的敏感数据！
cli-export-raw-warning-detail = 请确保输出目标安全,避免泄露密码、密钥等敏感信息。

# --- diff ---

cli-diff-title = 配置比对
cli-diff-pair = { $base } 与 { $overlay } 对比
cli-diff-identical = 两份配置完全相同
cli-diff-differ = 两份配置存在差异
cli-diff-base-heading = 基准({ $path }):
cli-diff-overlay-heading = 叠加({ $path }):
cli-diff-unified = 统一差异:
cli-diff-read-base-failed = 读取基准配置失败: { $path }
cli-diff-read-overlay-failed = 读取叠加配置失败: { $path }
cli-diff-base-format-unknown = 无法识别基准配置的格式
cli-diff-overlay-format-unknown = 无法识别叠加配置的格式

# --- snapshot ---

cli-snapshot-dir-missing = 快照目录不存在: { $directory }
cli-snapshot-in-dir = { $directory } 中的快照:
cli-col-timestamp = 时间戳
cli-col-filename = 文件名
cli-col-format = 格式
cli-snapshot-diff-need-two = 比对快照至少需要 2 份,实际只有 { $count } 份
cli-snapshot-diff-between = { $first } 与 { $second } 的差异
cli-snapshot-prune-invalid-duration = 无效的时长 '{ $value }': 应为天数,可带 'd' 后缀(例如 '30d')
cli-snapshot-prune-summary = 已清理 { $removed } 个早于 { $days } 天的快照(失败 { $failed } 个,跳过 { $skipped } 个)

# --- doctor ---

cli-doctor-load-ok = 已从 { $count } 个配置文件加载
cli-doctor-load-failed = 配置加载失败: { $message }
cli-doctor-skipped-not-loaded = 已跳过: 配置未加载
cli-doctor-schema-ok = 未发现 schema 问题
cli-doctor-schema-issues = 发现 { $count } 个 schema 问题: { $issues }
cli-doctor-sources-none = 没有任何配置源提供配置值
cli-doctor-sources-drifted = 以下配置源在不同键上的优先级不一致: { $sources } { $example }
cli-doctor-sources-env-below-file = env 优先级({ $env })不高于文件优先级({ $file }); env 覆盖将失效
cli-doctor-sources-ok = 各配置源的优先级链一致: { $sources }
cli-doctor-encryption-none = 未发现加密字段
cli-doctor-encryption-no-key = 发现 { $count } 个加密字段,但未设置 { $env_var }
cli-doctor-encryption-invalid-key = { $env_var } 不是有效的 32 字节十六进制密钥; 加密字段无法解密
cli-doctor-encryption-ok = { $count } 个加密字段解密成功
cli-doctor-encryption-failed = { $count } 个加密字段解密失败: { $fields }
cli-doctor-env-overrides-ok = 未发现冲突的环境变量覆盖
cli-doctor-env-overrides-conflicts = 发现 { $count } 处环境变量覆盖冲突: { $conflicts }
cli-doctor-header = 诊断(状态: { $status }, 退出码: { $code })

# --- agent 知识包 (docs --agent) ---

cli-docs-subcommand-inspect-purpose = 列出配置键及其来源+位置溯源信息
cli-docs-subcommand-validate-purpose = 结构性健壮性检查(null、数字形状的字符串)
cli-docs-subcommand-export-purpose = 打印合并后的配置(默认脱敏)
cli-docs-subcommand-diff-purpose = 比对两份配置文件
cli-docs-subcommand-snapshot-purpose = 管理配置快照
cli-docs-subcommand-schema-purpose = 为配置类型生成 JSON Schema,或从草稿生成 Rust 脚手架
cli-docs-subcommand-get-purpose = 查询单个配置键,输出单行 JSON
cli-docs-subcommand-doctor-purpose = 健康报告: schema、来源链、加密、环境变量覆盖冲突
cli-docs-subcommand-docs-purpose = 本知识包
cli-docs-global-args-config-file = --config <file> (可重复)
cli-docs-args-from-instance = --from-instance (从已加载实例逆向推导 schema 草稿)
cli-docs-args-from-schema = --from-schema <file> (从 JSON Schema 草稿生成 Rust 结构体脚手架)
cli-docs-exit-0 = 成功(doctor: 健康)
cli-docs-exit-1 = 配置错误 / doctor 有警告
cli-docs-exit-2 = I/O 错误 / doctor 有错误
cli-docs-contract-doctor = 单行 JSON: { $shape }; severity ok|warning|error
cli-docs-contract-get = 单行 JSON
cli-docs-contract-schema = 美化输出的 JSON Schema(2020-12 草案,--from-instance 用); --from-schema 输出 Rust 源码脚手架
cli-docs-recipe-load-env-overlay = 加载配置并叠加环境变量
cli-docs-recipe-ci-gate = 在 CI 中对配置执行质量门禁
cli-docs-recipe-find-source = 查找某个值的来源
cli-docs-recipe-get-value = 获取单个配置值
cli-docs-recipe-schema-reverse = schema-first 逆向工程
cli-docs-recipe-schema-adopt = 采用 schema-first
cli-docs-recipe-export-sanitized = 导出脱敏后的配置
cli-docs-markdown-title = # confers agent 知识包
cli-docs-markdown-subcommands-heading = ## 子命令
cli-docs-markdown-exit-codes-heading = ## 退出码
cli-docs-markdown-recipes-heading = ## 配方
cli-docs-markdown-subcommand = - **{ $name }** — { $purpose } (参数: { $args })
cli-docs-markdown-exit-0 = - `0` 成功(doctor: 健康)
cli-docs-markdown-exit-1 = - `1` 配置错误 / doctor 有警告
cli-docs-markdown-exit-2 = - `2` I/O 错误 / doctor 有错误
cli-docs-nothing-to-show = 没有可显示的内容: 使用 `confers docs --agent` 获取知识包
cli-export-wrote-file = 已导出配置到: { $path }
cli-export-wrote-annotated = 已导出带溯源标注的配置到: { $path }
cli-snapshot-none-after-sort = 排序后未找到任何快照
cli-snapshot-diff-not-enough = 没有足够的快照可供比较
cli-validate-missing-server-section = 服务器配置缺少 host/port
cli-validate-missing-database-section = 数据库配置缺少连接信息
cli-validate-null-value-indexed = 配置键 '{ $key }[{ $index }]' 的值为 null
cli-validate-null-value = 配置键 '{ $key }' 的值为 null
cli-validate-string-like-number = 键 '{ $key }' 的字符串值看起来像数字: { $value }
cli-validate-string-like-boolean = 键 '{ $key }' 的字符串值看起来像布尔值: { $value }
