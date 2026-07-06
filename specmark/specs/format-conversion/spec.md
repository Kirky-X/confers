# Spec — format-conversion

## Requirements

### R-format-conversion-001: 嵌套表/对象/映射的 map key 必须为 bare key

`toml_table_to_config_value` / `json_to_config_value` / `yaml_to_config_value` 在递归构造 `ConfigValue::Map` 时，map 的 key 必须是当前层级的 bare key（如 `"write_url"`），而非累积的 dotted path（如 `"database.write_url"`）。

**验收标准：**
- 输入 TOML `[database]\nwrite_url = "postgres://x"`，转换后 `ConfigValue::Map` 外层 key 为 `"database"`，内层 map key 为 `"write_url"`（不是 `"database.write_url"`）。
- 输入 JSON `{"database":{"write_url":"x"}}`，转换后内层 map key 为 `"write_url"`。
- 输入 YAML `database:\n  write_url: x`，转换后内层 map key 为 `"write_url"`。
- 三格式行为一致：同一嵌套结构经任一格式解析，产出的 `ConfigValue::Map` 结构相同（key 集合一致）。

### R-format-conversion-002: AnnotatedValue.path 保持 bare key

`AnnotatedValue.path` 字段保持当前语义（bare key），不改为 full dotted path。这是为了不破坏现有 `with_field_strategy("name", ...)` / `get_strategy(&low.path)` API。

**验收标准：**
- 嵌套表内层节点的 `AnnotatedValue.path` 等于该节点在源文件中的 key 名（如 `"write_url"`），不含父路径前缀。
- `with_field_strategy("write_url", MergeStrategy::Replace)` 能匹配到 `database.write_url` 字段的 path（因为 path 是 bare key `"write_url"`）。
- `all_paths_internal`（src/types.rs:732）输出不再出现重复前缀（如 `database.database.write_url`），应为 `["", "database", "database.write_url"]`。

### R-format-conversion-003: 数组元素路径构造不变

数组元素的 map key / path 仍使用索引（如 `"0"`, `"1"`），不受 bare key 修复影响。

**验收标准：**
- TOML `arr = [1, 2]` 转换后 `ConfigValue::Array` 元素的 `path` 为 `"arr.0"`, `"arr.1"`（保持现状）。
- 数组嵌套数组的 path 仍为 `"p.0.0"` 形式。

## Constraints

- **性能**：修复不引入额外分配。原代码 `Arc::from(path.clone())` 改为 `Arc::from(k.clone())`，分配次数不变。
- **向后兼容**：`AnnotatedValue.path` 语义不变，`with_field_strategy` API 不变。
- **一致性**：三格式（TOML/JSON/YAML）修复方式同构，不允许只修一种。

## Out of Scope

- INI 格式的 dotted key 行为（`parse_ini` 用 `format!("{}.{}", sec, k)` 作为 key）— INI 无嵌套 struct 场景。
- `AnnotatedValue.path` 升级为 full dotted path 的提案 — 独立变更。
- `value_to_json` 重构 — 该函数已正确使用 map key，无需改动。
