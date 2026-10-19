# confers-macros

[confers](https://github.com/Kirky-X/confers) 仓库的过程宏子 crate（目录 `confers/macros`）。

## 与父 crate 的关系

| 项 | 内容 |
|---|---|
| 所属仓库 | [confers](https://github.com/Kirky-X/confers)（[crates.io](https://crates.io/crates/confers) · [docs.rs](https://docs.rs/confers)） |
| 依赖方向 | `confers` → `confers-macros`（父 crate 以 `path + version` 双写依赖本 crate） |
| 是否 re-export | 是：父 crate 在 crate 根部导出 `Config`、`ConfigSchema`、`ConfigMigration`、`ConfigModules`、`ConfigClap` 五个派生宏，使用者无需直接依赖本 crate |
| 适用场景 | 仅在需要绕过父 crate 的 re-export 面（如自建宏包装、编译期代码生成工具）时才直接依赖本 crate |

## 提供的派生宏

| 宏 | 作用 |
|---|---|
| `#[derive(Config)]` | 为结构体生成多来源配置加载、默认值注入与环境变量覆盖 |
| `#[derive(ConfigSchema)]` | 生成配置的 JSON Schema |
| `#[derive(ConfigMigration)]` | 生成配置版本迁移逻辑 |
| `#[derive(ConfigModules)]` | 生成按模块聚合的配置入口 |
| `#[derive(ConfigClap)]` | 生成与 clap 命令行参数的双向映射 |

## 使用

推荐经父 crate 使用（无需在 `Cargo.toml` 中单独声明本 crate）：

```rust
use confers::Config;

#[derive(Config, Debug)]
#[config(prefix = "app")]
struct AppConfig {
    #[config(default = "8080")]
    port: u16,
}
```

若确需直接依赖，按父 crate 的当前版本号声明（两者始终同号，见 [crates.io](https://crates.io/crates/confers-macros)）。

## 版本与支持

- 版本经 `version.workspace = true` 随仓库统一，与 `confers` 同号发布。
- 仅维护当前 minor 版本线，详见 [confers 安全文档 · 支持的版本](https://github.com/Kirky-X/confers/blob/main/docs/SECURITY.md#-支持的版本与维护策略)。

## 许可证

MIT
