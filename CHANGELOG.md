# Changelog

本文件记录 confers 的显著变更（格式参考 Keep a Changelog；版本遵循 SemVer 的 rc 预发布约定）。

## [0.6.0-rc.6] - Unreleased

### 变更（破坏面声明）

- **`#[config(validate)]` 保持兼容 no-op**：校验辅助方法 `confers_validate()` 改由新增的
  opt-in 属性 `#[config(validate_helper)]` 生成（要求 `validation` feature 与
  `#[derive(garde::Validate)]`）。两种属性都不会把校验自动挂进加载管线。
  旧代码（仅设置 `validate`）升级后行为与编译结果不变。
- **`CorsValidator` 不再支持 unit-struct 裸名构造**
  （`let v: CorsValidator = CorsValidator;` 编译失败）：为支持
  `with_origins_key` / `with_methods_key` / `with_max_age_key` 自定义键构造器而字段化。
  `CorsValidator::new()` 与 `Default` 行为不变（默认键名与旧版逐字节一致）。
  经检索 confers / mnemis / sdforge 生态均仅以 `::new()` 构造，无实际消费者受影响。
- **`WatcherGuard::shutdown` 异常路径行为变更**：被等待的任务 panic 时（join 返回
  `Err`）现在记录错误并返回 `Ok(false)`；旧版把 panic 误报为 `Ok(true)`。
  超时与干净完成的语义不变。

### 新增

- `WatcherGuard::with_task` / `set_task_handle` 由 `pub(crate)` 放宽为 `pub`；
  `shutdown` 文档注明超时返回 `Ok(false)` 的降级语义（任务不会被取消，仍在后台运行）。
- `ConfigBuilder::env_source(EnvSource)`：接收预构建的 `EnvSource`（与 `env_prefix` /
  `env_separator` 的组合语义见 rustdoc）。
- `hot-reload-kit` feature（默认关，不入 recommended/production/full 预置）+
  `HotReloader` 门面：FsWatcher + 渐进重载 + watch 广播 + 优雅停机的装配形态；
  loader 经 spawn_blocking 卸载，panic 计入失败计数。
- `ProgressiveReloader::with_pre_commit_check` + `PreCommitCheck` trait（`Err` 即拒）
  与新错误变体 `ConfigError::ReloadRejected`（受 `#[non_exhaustive]` 保护）。
- `RemappedConfigProvider` 键重映射视图 + `JwtSecretValidator::with_secret_key` /
  `CorsValidator::with_{origins,methods,max_age}_key`（默认键名不变）。
  注意：CORS 三键仅部分重映射会触发既有规则的稳定误报（详见 `CorsValidator` rustdoc），
  建议仅 `JwtSecretValidator` 起步。
