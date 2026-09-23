# Spec — masking-audit

> Main spec for capability `masking-audit`.

## Requirements

### R-mask-001: 快照脱敏接线

builder 自动快照传入类型敏感路径列表；sensitive 字段在快照中为 `[REDACTED]`；快照文件权限 0600。
**验收标准：**
- e2e：`#[config(sensitive=true)]` 字段值不出现在快照 JSON 明文
- 快照文件 mode 断言 0600（unix）

### R-mask-002: CLI 默认脱敏

`inspect`（text/json）与 `get` 默认按字段名规则掩码；`--reveal` 显式明文并输出 stderr 警告。
**验收标准：**
- `get db.password` 默认输出掩码；`--reveal` 输出原文
- inspect json 模式敏感路径值为掩码

### R-mask-003: 脱敏规则单一来源

`sanitize_error_message` 并入字段名驱动规则（`password=xxx` 值掩码）；`user_message()` 真实调用 sanitize；i18n 错误模板不再内插明文 message。
**验收标准：**
- `password = "hunter2"` 经 export sanitize 被掩
- 含 `user:pass@host` 的 InvalidValue 经 user_message 输出脱敏 URL
- errors.ftl 相关模板无 `{ $message }` 明文插值

### R-mask-004: 冲突报告脱敏

conflict_report 的 low/high value 按 sensitive_paths 掩码后入报告。
**验收标准：**
- 敏感字段冲突报告中值为掩码而非 `format!("{:?}")` 原文

### R-mask-005: 敏感词表统一

`security/patterns.rs` 为唯一判定来源：增补 authorization/passwd/pwd/dsn/bearer、复数形命中（passwords/credentials/tokens/keys）、`monkey` 不误报；audit 关键词判定调用 patterns。
**验收标准：**
- 命中/不命中用例表全部通过；audit 与 security 对同一字段名结论一致

### R-mask-006: 掩码定长化

`mask_value` 输出固定 8 星号，不泄露原文长度与前缀。
**验收标准：**
- 3/7/32 字符值掩码输出一致为 `********`

### R-mask-007: 审计链加固

AuditSink 收到 sanitize 后事件；`verify_audit_chain` 常量时间比较；`AuditConfig::with_hmac_key()` 支持外置 HMAC 密钥（缺省沿用 salt 并在文档标注威胁模型）。
**验收标准：**
- sink 事件 key 名已脱敏（与落盘一致）
- 逐字节比较无短路（构造用例或代码审查记录）
- 外置密钥时文件头不存密钥且验证通过；篡改/删除/重排既有测试不回归

### R-mask-008: 文档对齐

五份文档与 CHANGELOG 反映最终实现：优先级语义、env 嵌套示例、渐进发布措辞、熔断覆盖面、总线可靠性等级、审计威胁模型、密钥 env 名、全部行为变更清单。
**验收标准：**
- 文档中不再存在与实现矛盾的陈述（审计引用的 ARCHITECTURE.md:25/157、API_REFERENCE.md:104/108/122、USER_GUIDE.md:255-260/934、SECURITY.md:155/313 均修正）
- typos/pre-commit 全量通过

## Constraints

- `ConfigValue` 的 Debug 保持明文（库内调试用途），在文档标注；面向用户路径（CLI/报告/快照）全部脱敏。
- 审计链既有 HMAC 覆盖与 tamper/deletion/reorder 测试不得回归。

## Out of Scope

- 审计日志外部化存储、SIEM 集成、结构化审计查询 API。
