# Spec — env-source

> Delta spec for change `fix-0.4.1-nested-deserialization`. 覆盖此变更引入/修改的该能力域需求。

## Requirements

### R-env-source-001: EnvSource 必须对环境变量值做确定性类型推断

`EnvSource::collect()` 在将环境变量字符串转为 `ConfigValue` 时，必须按以下**显式顺序**尝试推断，首个成功的结果即为最终类型：

1. **bool**：`s.eq_ignore_ascii_case("true")` → `ConfigValue::Bool(true)`；`s.eq_ignore_ascii_case("false")` → `ConfigValue::Bool(false)`。
2. **i64**：`s.parse::<i64>()` 成功 → `ConfigValue::I64(i)`。
3. **u64**：`s.parse::<u64>()` 成功 → `ConfigValue::U64(u)`（覆盖 i64 溢出的大正整数）。
4. **f64**：仅当 `s.contains('.') || s.contains('e') || s.contains('E')` 时尝试 `s.parse::<f64>()`，成功 → `ConfigValue::F64(f)`。
5. **fallback**：`ConfigValue::String(s.to_string())`。

**验收标准：**
- `"true"` → `ConfigValue::Bool(true)`；`"FALSE"` → `ConfigValue::Bool(false)`；`"True"` → `ConfigValue::Bool(true)`。
- `"5432"` → `ConfigValue::I64(5432)`；`"-7"` → `ConfigValue::I64(-7)`。
- `"18446744073709551615"`（u64::MAX）→ `ConfigValue::U64(18446744073709551615)`。
- `"3.14"` → `ConfigValue::F64(3.14)`；`"1e10"` → `ConfigValue::F64(1e10)`；`"1E-3"` → `ConfigValue::F64(1E-3)`。
- `"hello"` → `ConfigValue::String("hello")`；`""` → `ConfigValue::String("")`。
- `"123abc"` → `ConfigValue::String("123abc")`（解析失败保留字符串）。
- `"123"` → `ConfigValue::I64(123)`（不含 `.`/`e`，不尝试 f64，避免把整数变浮点）。

### R-env-source-002: 类型推断必须同时作用于 .env 文件路径和真实环境变量路径

`EnvSource::collect()` 中两条路径（`dotenvy::dotenv_iter()` 读取的 .env 条目，和 `std::env::vars()` 读取的真实环境变量）都必须经过 `infer_config_value`，不能只对其中一条应用。

**验收标准：**
- 设置真实环境变量 `TESTCFG_PORT=5432`，`EnvSource::with_prefix("TESTCFG_").collect()` 后 map→"port"→inner 为 `ConfigValue::I64(5432)`。
- 写入 .env 文件 `TESTCFG_NUM=42`，`EnvSource::with_prefix("TESTCFG_").collect()` 后 map→"num"→inner 为 `ConfigValue::I64(42)`。
- 两条路径产出的数值字段都能被 `serde_json::from_value` 反序列化为 `u32`/`u64`/`i32` 等。

### R-env-source-003: _FILE 后缀读取的文件内容也走类型推断

`resolve_value` 返回的文件内容字符串（Docker secrets 场景）也必须经过 `infer_config_value`，使 `MY_PORT_FILE=/path/to/file`（文件内容为 `"8080"`）能反序列化为数值字段。

**验收标准：**
- 创建临时文件内容为 `"8080"`，设置 `MYTEST_PORT_FILE=<path>`，`EnvSource::with_prefix("MYTEST_").collect()` 后 map→"port"→inner 为 `ConfigValue::I64(8080)`。
- 文件内容为 `"true"` 时 → `ConfigValue::Bool(true)`。

## Constraints

- **确定性**（Rule 5）：推断顺序是硬编码的，不读取配置、不调用模型、不暴露开关。
- **向后兼容**：声明为 `String` 的字段，若环境变量值能解析为数值，修复后 serde 会拒绝把数值反序列化为 String — 这是可接受的破坏（依赖 bug 副作用的用法）。CHANGELOG 需注明。
- **不引入新依赖**：只用 `str::parse`、`eq_ignore_ascii_case`、`contains`，不引入 `regex` 或其他解析库。
- **性能**：每个环境变量值最多尝试 4 次 `parse`（bool 两次字符串比较 + i64 parse + u64 parse + 可能的 f64 parse），失败则 fallback。可接受。

## Out of Scope

- 类型推断可配置开关（Rule 2 简洁优先）。
- 自定义类型推断顺序（Rule 5 确定性逻辑）。
- 环境变量值的嵌套 JSON/YAML 解析（如 `APP_CONFIG='{"a":1}'` 解析为 map）— 独立提案。
