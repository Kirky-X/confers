# Design — fix-0.4.1-nested-deserialization

## Context

confers 的反序列化路径为：`Source::collect() → AnnotatedValue(ConfigValue::Map) → value_to_json() → serde_json::Value → serde_json::from_value() → T`。

`ConfigValue::Map(Arc<IndexMap<Arc<str>, AnnotatedValue>>)` 的 key 是 serde 反序列化 struct 字段时的匹配依据——`value_to_json` 直接把 map key 当作 JSON object key，serde 用它去匹配 `#[derive(Deserialize)]` struct 的字段名。

当前 `toml_table_to_config_value` / `json_to_config_value` / `yaml_to_config_value` 在递归构造嵌套 map 时，把「累积 dotted path」（如 `"database.write_url"`）作为内层 map 的 key，而非 bare key（如 `"write_url"`）。这导致内层 map 的 key 与目标 struct 的字段名不匹配，serde 报 `missing field`。

`EnvSource::collect()` 则把所有环境变量值存为 `ConfigValue::String`，`value_to_json` 转成 `serde_json::Value::String`，`serde_json::from_value` 拒绝把 JSON string 反序列化为 `u32`/`bool` 等数值类型。

`merger::merge_maps_with_cow` 已按 bare key 匹配（`low.get(k)`），不依赖 dotted key——这反向证明 dotted key 是 bug 而非设计。

## Decision

### Decision 1: convert.rs 三个转换函数使用 bare key 作为 map key，path 字段保持 bare key 不变

修改 `toml_table_to_config_value` / `json_to_config_value` / `yaml_to_config_value`：

```rust
// 修改前（bug）
(
    Arc::from(path.clone()),  // map key = dotted path（错误）
    AnnotatedValue::new(cv, source.clone(), k.clone()),  // path = bare key
)

// 修改后（fix）
(
    Arc::from(k.clone()),  // map key = bare key（与 struct 字段名匹配）
    AnnotatedValue::new(cv, source.clone(), k.clone()),  // path = bare key（保持现状）
)
```

- **只改 map key**：从 dotted path 改为 bare key，使 serde 能按 struct 字段名匹配，嵌套反序列化成功。
- **`AnnotatedValue.path` 保持 bare key**：现有 `with_field_strategy("name", ...)` / `get_strategy(&low.path)` API 依赖 path 是 bare key（`engine.rs:55`、`chain.rs:77`）。改为 full dotted path 会破坏该 API（用户需改用 `"database.name"` 而非 `"name"`），属于不必要的破坏性变更。
- **`all_paths_internal`（types.rs:732）不受影响**：该函数从 `current_path + "." + key` 重建 dotted path，不依赖子节点的 `path` 字段；且它期望 map key 是 bare key（当前 dotted key 反而导致路径重复如 `database.database.write_url`）。修复后 `all_paths` 输出反而正确。
- **数组索引路径**（如 `"p.0"`）：数组元素无「字段名」概念，path 保持索引形式不变；本次只改 map 的 key，不动 array 元素的 path 构造。

### Decision 2: EnvSource 增加确定性类型推断（显式顺序代码，Rule 5）

在 `EnvSource::collect()` 中，对每个解析后的字符串值调用新增的 `infer_config_value(s: &str) -> ConfigValue`：

```rust
fn infer_config_value(s: &str) -> ConfigValue {
    // 1. bool（精确匹配 "true"/"false"，大小写不敏感）
    if s.eq_ignore_ascii_case("true") { return ConfigValue::Bool(true); }
    if s.eq_ignore_ascii_case("false") { return ConfigValue::Bool(false); }
    // 2. i64（先试有符号，范围超 i64 再试 u64）
    if let Ok(i) = s.parse::<i64>() { return ConfigValue::I64(i); }
    if let Ok(u) = s.parse::<u64>() { return ConfigValue::U64(u); }
    // 3. f64（仅当含 '.' 或 'e' 时尝试，避免把 "123" 解析成 123.0）
    if s.contains('.') || s.contains('e') || s.contains('E') {
        if let Ok(f) = s.parse::<f64>() { return ConfigValue::F64(f); }
    }
    // 4. fallback: 字符串
    ConfigValue::String(s.to_string())
}
```

推断顺序写成显式代码（bool → i64 → u64 → f64 → string），不交给模型/配置决定（Rule 5）。`_FILE` 后缀读取的文件内容也走同一推断。

### Decision 3: 版本号 0.4.0 → 0.4.1

`Cargo.toml`：
- `[package] version = "0.4.1"`
- `[workspace.package] version = "0.4.1"`
- `confers-macros = { version = "0.4.1", path = "macros" }`

`macros/Cargo.toml`：`version = "0.4.1"`（若 macros 自身版本也是 0.4.0，同步升；否则只升主包依赖声明）。

## Alternatives Considered

### A1: 只修 TOML，不修 JSON/YAML（用户只报了 TOML）
**否决**。三处函数同构，只修一处会留下两份已知 bug 代码，违反 Rule 7（暴露冲突不折中）与 Rule 11（惯例优先于新颖——三处应保持一致）。且 JSON/YAML 嵌套 struct 反序列化同样失败，只是用户尚未触发。

### A2: 在 `value_to_json` 里 split dotted key 重建嵌套 object
**否决**。这把 bug 修复点放到了下游，convert.rs 仍产出错误结构；merger 在合并阶段会比较 map key，dotted key 与 EnvSource 的 bare key 无法匹配，合并结果错误。修复应在上游（convert.rs）。

### A3: 用 serde 的 `deserialize_with` 自定义反序列izer 处理 string→number
**否决**。这要求用户在 struct 上加属性宏，违反「zero boilerplate」卖点。类型推断应在 library 内部完成，对用户透明。

### A4: EnvSource 不做类型推断，改 `value_to_json` 让 JSON string 能反序列化为数值
**否决**。`serde_json::from_value` 是标准库行为，无法让 `Value::String("5432")` 反序列化为 `u32` 而不破坏类型安全。`value_to_json` 应忠实反映 ConfigValue 类型；推断应在 EnvSource 入口完成。

### A5: EnvSource 类型推断做成可配置（builder 方法开关）
**否决**（Rule 2 简洁优先 + Rule 5 确定性逻辑）。推断顺序是确定的，无需配置；用户需要原始字符串时声明 `String` 字段即可。

## Consequences

**正面：**
- 文件加载嵌套 struct 反序列化可用（核心场景修复）。
- 环境变量数值/布尔字段反序列化可用。
- merger 合并 FileSource + EnvSource 时 key 一致，合并结果正确。
- `all_paths_internal` 不再产生重复前缀路径（如 `database.database.write_url`），路径输出正确。

**负面/破坏性：**
- `convert.rs` 中 7 处单元测试依赖 dotted-key 行为，需同步更新（已在 Scope 列出）。
- 极少数用户若依赖「环境变量数字值反序列化到 String 字段」的 bug 副作用，修复后会失败——需在 CHANGELOG 注明，建议用户把字段类型改为数值或显式 quote 环境变量值。
- `AnnotatedValue.path` 保持 bare key 不变（无破坏），但「同名字段在不同嵌套层级共享同一 path」的限制仍在（如 `database.name` 和 `server.name` 的 path 都是 `"name"`），`with_field_strategy("name", ...)` 会同时匹配两者。这是既有行为，本次不改变。

**技术债：**
- INI 格式仍用 dotted key（`format!("{}.{}", sec, k)`），与修复后的三格式不一致。本次不修（INI 无嵌套 struct 场景），但应在 docs 注明 INI 的 key 语义与其他格式不同。
- `AnnotatedValue.path` 仍为 bare key，未来若需精确定位嵌套字段（如 `database.write_url`），需独立提案升级 path 语义并同步 `with_field_strategy` API。
