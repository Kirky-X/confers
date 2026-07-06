# Tasks — fix-0.4.1-nested-deserialization

## Phase 1: Bug 1 — convert.rs 嵌套表 map key 修复（TOML/JSON/YAML）

- [x] [T001] [P0] [Red] 在 src/impl_/convert.rs 的 `mod tests` 新增三个失败测试：`test_toml_nested_table_uses_bare_key`（构造 `[database]\nwrite_url="x"` 的 toml::Table，调用 `toml_table_to_config_value`，断言内层 map key 为 `"write_url"` 而非 `"database.write_url"`）、`test_json_nested_object_uses_bare_key`（同理 JSON `{"database":{"write_url":"x"}}`）、`test_yaml_nested_mapping_uses_bare_key`（同理 YAML）。运行 `cargo test --features toml,json,yaml --lib convert` 确认三测试失败。
- [x] [T002] [P0] [Green] 修复 src/impl_/convert.rs:27 `toml_table_to_config_value`：将 `Arc::from(path.clone())` 改为 `Arc::from(k.clone())`（map key 用 bare key）；`AnnotatedValue::new` 第三参数保持 `k.clone()`（path 仍为 bare key）。运行 T001 三测试确认通过。
- [x] [T003] [P0] [Green] 修复 src/impl_/convert.rs:104 `json_to_config_value` 的 `Value::Object` 分支：将 `Arc::from(p.clone())` 改为 `Arc::from(k.clone())`。运行 `cargo test --features json --lib convert::tests::test_json_nested_object_uses_bare_key` 确认通过。
- [x] [T004] [P0] [Green] 修复 src/impl_/convert.rs:153 `yaml_to_config_value` 的 `Value::Mapping` 分支：将 `Arc::from(p.clone())` 改为 `Arc::from(key)`。运行 `cargo test --features yaml --lib convert::tests::test_yaml_nested_mapping_uses_bare_key` 确认通过。
- [x] [T005] [P1] 更新 src/impl_/convert.rs 中 7 处依赖 dotted-key 行为的既有测试断言，全部改为断言 bare key：`test_toml_value_table_delegates_to_table_fn`（L266 `map.get("p.a")` → `map.get("a")`）、`test_toml_table_with_prefix`（L307 `map.get("root.k")` → `map.get("k")`）、`test_toml_table_nested_path_construction`（L330 `map.get("root.inner")` → `map.get("inner")`；L333 `inner_map.get("root.inner.x")` → `inner_map.get("x")`）、`test_json_object_with_prefix`（L428 `map.get("root.a")` → `map.get("a")`）、`test_json_object_nested_path`（L440 `inner_map.get("outer.inner")` → `inner_map.get("inner")`）、`test_yaml_mapping_with_prefix`（L542 `map.get("root.a")` → `map.get("a")`）、`test_yaml_mapping_nested_path`（L554 `inner_map.get("outer.inner")` → `inner_map.get("inner")`）。
- [x] [T006] [P0] [Red] 新增端到端回归测试文件 tests/integration_nested_deserialize.rs：定义 `#[derive(Deserialize, PartialEq, Debug)] struct DbConfig { host: String, port: u16 }` 和 `struct AppConfig { database: DbConfig }`；写临时 TOML/JSON/YAML 文件含嵌套表，`ConfigBuilder::<AppConfig>::new().file(tmp).build()` 成功且 `config.database.host == "localhost"`、`config.database.port == 5432`。运行 `cargo test --test integration_nested_deserialize --features toml,json,yaml` 确认失败。
- [x] [T007] [P0] 验证：`cargo test --features toml,json,yaml` 全部通过（含 T006 端到端测试现在通过）。提交：`git commit -m "fix(convert): use bare key as map key for nested tables/objects/mappings"`。

## Phase 2: Bug 2 — EnvSource 类型推断

- [x] [T008] [P0] [Red] 在 src/impl_/config/source.rs 的 `mod tests` 新增 `test_infer_config_value`：断言 `infer_config_value("true")==ConfigValue::Bool(true)`、`("false")==Bool(false)`、`("5432")==I64(5432)`、`("-7")==I64(-7)`、`("18446744073709551615")==U64(u64::MAX)`、`("3.14")==F64(3.14)`、`("1e10")==F64(1e10)`、`("hello")==String("hello")`、`("")==String("")`、`("123abc")==String("123abc")`。运行确认函数不存在编译失败。
- [x] [T009] [P0] [Green] 在 src/impl_/config/source.rs 的 `impl EnvSource` 块内新增 `fn infer_config_value(s: &str) -> ConfigValue`，按 design.md Decision 2 的显式顺序实现：bool（`eq_ignore_ascii_case`）→ i64（`parse::<i64>()`）→ u64（`parse::<u64>()`）→ f64（仅当含 `.`/`e`/`E` 时 `parse::<f64>()`）→ fallback `ConfigValue::String`。运行 T008 测试确认通过。
- [x] [T010] [P0] [Red] 在 src/impl_/config/source.rs 新增两个 `#[serial_test::serial]` 测试：(a) `test_env_source_collect_infers_types`：`std::env::set_var("TESTCFG_PORT", "5432")`、`("TESTCFG_DEBUG", "true")`、`("TESTCFG_HOST", "localhost")`；`EnvSource::with_prefix("TESTCFG_").collect()`；断言 map→"port"→inner 是 `ConfigValue::I64(5432)`、"debug"→`Bool(true)`、"host"→`String("localhost")`；cleanup。(b) `test_env_source_file_suffix_infers_type`：写临时文件内容 `"8080"`，`std::env::set_var("MYTEST_PORT_FILE", path)`；`EnvSource::with_prefix("MYTEST_").collect()`；断言 map→"port"→inner 是 `ConfigValue::I64(8080)`；cleanup。运行确认失败（当前全为 String，覆盖 specs/env-source R-003）。
- [x] [T011] [P0] [Green] 修改 src/impl_/config/source.rs:329 和 :346 两处 `ConfigValue::String(resolved)`，改为 `Self::infer_config_value(&resolved)`（.env 路径和真实 env 路径都改，`resolved` 来自 `resolve_value` 故 _FILE 内容也走推断）。运行 T010 两测试确认通过。
- [x] [T012] [P0] [Red] 新增端到端测试 tests/integration_env_types.rs：定义 `struct TypedConfig { port: u32, debug: bool, host: String }`；`std::env::set_var("APP_PORT","8080")`、`("APP_DEBUG","true")`、`("APP_HOST","localhost")`；`ConfigBuilder::<TypedConfig>::new().env_prefix("APP_").build()` 成功且 `config.port==8080`、`config.debug==true`、`config.host=="localhost"`；cleanup。运行 `cargo test --test integration_env_types --features env` 确认失败。
- [x] [T013] [P0] 验证：`cargo test --features env` 全部通过。提交：`git commit -m "fix(env): infer bool/int/float types from env var strings"`。

## Phase 3: 版本号与全量验证

- [x] [T014] [P1] 修改 Cargo.toml：`[package] version = "0.4.0"` → `"0.4.1"`（L3）；`[workspace.package] version = "0.4.0"` → `"0.4.1"`（L70）；`confers-macros = { version = "0.4.0", path = "macros" }` → `"0.4.1"`（L30）。运行 `cargo build` 确认 confers-macros 依赖版本解析成功。
- [x] [T015] [P0] 全量验证：`cargo test --all-features`（全特性）、`cargo test --no-default-features --features minimal`（最小特性）、`cargo test --no-default-features --features tol,json,env`（默认特性子集）、`cargo clippy --all-features -- -D warnings`（lint 零警告）。任一失败则回滚到对应 Phase 修复。
- [ ] [T016] [P1] 提交版本号变更：`git commit -m "chore: bump version to 0.4.1"`。

## Phase 4: Convergence
（仅由 /specmark converge 追加，propose 不写本节）
