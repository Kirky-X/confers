# Spec — config-loading

> Delta spec for change `fix-audit-defects-r1`. 覆盖此变更引入/修改的多来源加载、合并与宏生成行为需求。

## Requirements

### R-loading-001: 同优先级按声明顺序合并
多来源合并时同优先级来源保持声明顺序（后声明覆盖先声明）；DefaultSource 优先级低于文件/env/memory 全部来源，任何文件名下默认值都不覆盖文件值。
**验收标准：**
- 文件与 default 同配时文件值胜（任意文件名，含 `config.toml`）
- 先 `.file(mmm)` 后 `.file(bbb)` 时 bbb 值胜
- memory 与 env 同优先级（50）按声明顺序而非 id 字母序
- `tests/core/merge.rs` 不再需要 priority(20) 绕过注释

### R-loading-002: env 嵌套冲突确定性报错
`insert_nested` 标量/嵌套路径前缀冲突返回携带变量名与路径的错误；收集顺序先排序，结果与 `std::env::vars()` 迭代顺序无关。
**验收标准：**
- `X_DB=标量` + `X_DB_HOST=v` 共存 → ConfigError 含 `X_DB` 与路径
- 逆序注入相同变量集 → 相同错误（确定性）

### R-loading-003: 宏 env 注入最小化与默认值生效
`load_file_with_env` 仅注入显式声明 env；`load_file`/`load_file_with_env` 为全部 `#[config(default)]` 字段注册默认值。
**验收标准：**
- `deny_unknown_fields` 结构体在 PATH/HOME 等无关变量存在时构建成功
- 前缀变量仍覆盖声明字段
- 部分 TOML 缺字段时 `load_file` 以默认值成功构建

### R-loading-004: null 覆盖语义统一
map 内叶子合并的 Null 行为与根级一致：高优先级显式 null 不覆盖已有值。
**验收标准：**
- 根级与 map 内同构测试均断言 null 不覆盖
- Replace 策略显式配置时行为仍由策略决定（不回归 T008）

### R-loading-005: env 类型错误可定位
env 值类型与目标字段不符的 serde 错误携带完整字段路径（无空 key）。
**验收标准：**
- String 字段遇 `FOO=1.0` 错误信息含 `foo` 路径

### R-loading-006: env_separator 可配置
ConfigBuilder/SourceChainBuilder 提供 `env_separator`；文档语义 `APP_DB__HOST → db.host` 可用。
**验收标准：**
- `env_separator("__")` 后嵌套映射测试通过

### R-loading-007: merge_strategy 与 profile 真实生效
字段级 merge_strategy 在嵌套路径生效；profile/profile_env 生成 `<stem>.<env>.<ext>` 叠加（缺文件跳过，env 缺省 RUN_ENV）。
**验收标准：**
- Append/Replace 策略测试通过且无策略路径行为不变
- prod 叠加文件覆盖基础字段、无叠加文件时行为同现状

### R-loading-008: 插值转义与敏感告警
`$${VAR}` 输出字面 `${VAR}`；插值 key 使用 serde 名（serde(rename) 后生效）；嵌套字段参与插值；非敏感字段引用 sensitive_vars 产生 warning（tracing + 校验列表）。
**验收标准：**
- 三项插值行为测试 + 敏感引用告警测试通过

### R-loading-009: 引擎一致性
`MergeEngine::merge` 尊重入口 priority（低优先级值不因参数位置反杀）；`report_conflict` winner 与实际胜者一致；宏 default 空值/None 编译可用；宏敏感 `_FILE` 坏路径报错。
**验收标准：**
- priority=10 low 胜 priority=5 high 且报告 winner=Low
- 两种 default 写法编译运行通过
- 坏 `_FILE` 路径返回错误

## Constraints
- 全部行为变更记录 CHANGELOG；默认值优先级变化对既有测试的影响须逐个核实而非批量改断言。
- clippy `-D warnings` 干净；MSRV 1.97.1。

## Out of Scope
- 服务端配置中心、长连接推送、profile 组合语义扩展（多 profile 叠加）。
