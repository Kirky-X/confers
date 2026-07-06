# fix-0.4.1-nested-deserialization

## Motivation

confers 0.4.0 存在两个阻塞性 bug，导致下游项目（引用方）的 T017 任务无法完成：

1. **FileSource 嵌套表反序列化失败**：TOML/JSON/YAML 解析器在转换嵌套表时，把累积的 dotted path（如 `"database.write_url"`）当作 `ConfigValue::Map` 的 key，而非 bare key（如 `"write_url"`）。serde 在反序列化嵌套 struct 时，在内层 map 找不到与字段名匹配的 bare key，导致 `missing field` 错误。这使得任何形如 `#[derive(Deserialize)] struct Config { database: Database }` 的嵌套结构都无法从文件加载。

2. **EnvSource 无类型转换**：环境变量值都是字符串，`EnvSource::collect()` 一律存为 `ConfigValue::String`。当目标 struct 含 `u32`/`f64`/`bool` 等非字符串字段时，`serde_json::from_value` 拒绝将 JSON 字符串反序列化为数值类型，导致 `invalid type: string "5432", expected u32` 错误。

两个 bug 都使 confers 在最常见使用场景（文件 + 环境变量 → 类型化 struct）下不可用，必须修复并发布 0.4.1。

## Scope

- **修复 Bug 1**：`src/impl_/convert.rs` 中三个转换函数 `toml_table_to_config_value` / `json_to_config_value` / `yaml_to_config_value` 的 map key，从 dotted `path` 改为 bare key。`AnnotatedValue.path` 字段保持 bare key 不变（不破坏现有 `with_field_strategy` API，详见 design Decision 1）。
- **修复 Bug 2**：`src/impl_/config/source.rs` 中 `EnvSource::collect()` 增加类型推断，按 `bool → i64/u64 → f64 → string` 顺序尝试解析，失败则保留为字符串。
- **更新受影响测试**：`src/impl_/convert.rs` 单元测试中依赖 dotted-key 行为的断言（7 处）改为断言 bare key。
- **新增回归测试**：覆盖「文件 → 嵌套 struct」与「环境变量 → 数值字段」端到端反序列化。
- **版本号**：`Cargo.toml` 中 `version` 与 `workspace.package.version` 从 `0.4.0` → `0.4.1`；`confers-macros` 依赖版本同步。

## Non-Goals

- **不重构 `value_to_json`**：该函数已正确使用 map key 作为 JSON object key，bug 在上游转换函数；改动 `value_to_json` 无必要且会扩大影响面。
- **不修改 merger**：`merge_maps_with_cow` 已按 bare key 匹配，修复 convert 后 merger 行为反而更正确；改动 merger 会引入回归风险。
- **不引入「env 类型推断可配置」**：推断策略写成显式顺序代码（Rule 5），不暴露开关；用户需要原始字符串时可显式声明 `String` 字段。
- **不修复 INI 格式**：INI 解析器（`parse_ini`）已用 `format!("{}.{}", sec, k)` 作为 key，但 INI 本身是扁平 key-value 格式，无嵌套 struct 场景，不在本次范围。
- **不修改 `MemorySource` / `DefaultSource`**：它们通过 `insert_nested` 已正确构建 bare-key 嵌套结构，无此 bug。

## Clarifications

- **[scope]** Q: 用户只报告了 TOML 的 bug，JSON/YAML 是否也要修？
  A: 是。三处转换函数有完全相同的 bug 模式（同一份代码模板复制），修复只改 TOML 会留下不一致代码库（Rule 7 冲突暴露）。已确认 `json_to_config_value` 第 95-113 行、`yaml_to_config_value` 第 143-163 行与 TOML 版本同构。统一修复。

- **[technical]** Q: EnvSource 类型推断是否会破坏现有「字段就是 String」的用法？
  A: 不会。推断顺序为 bool → i64/u64 → f64 → string，只有当字符串能被解析为对应类型时才转换。声明为 `String` 的字段在推断为数值后，serde 反序列化 `String` 字段时 serde_json 会拒绝（保持原行为）；声明为数值的字段才能受益。需要注意：原本「环境变量值是数字但字段是 String」的用法若依赖 serde_json 的 string→string 透传，修复后该值会变成 JSON number，serde 反序列化为 `String` 字段会失败。这是可接受的破坏：原本这种用法依赖的是 bug 副作用，且用户应使用 `String` 字段时环境变量值本就应是字符串语义。

## NEEDS CLARIFICATION

无。所有需求已转为具体任务。
