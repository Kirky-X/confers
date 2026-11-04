# confers v0.6.0-rc.13 发布报告

**发布日期**：2026-11-04
**发布版本**：`0.6.0-rc.13`（上一版 `0.6.0-rc.12`）
**发布提交**：`b2077f1`（tag `v0.6.0-rc.13`）
**执行方式**：手动编排（release-pipeline 18 阶段门禁）+ specmark 变更追踪（`specmark/changes/release-v0-6-0-rc13/`）

---

## 1. 发布内容

| 类型 | 内容 | 来源提交 |
| --- | --- | --- |
| 修复 | **R-watch-008 文件监听建立竞态**：`FsWatcher::new()` / `MultiFsWatcher::new()` 现在同步建立 OS 级 watch，`Ok` 返回即监听生效；建立失败返回 `Err(ConfigError::WatcherError)` | `2995446` |
| 修复 | pulldown-cmark 0.13.6（RUSTSEC-2026-0433） | `8c27a58` |
| 依赖 | serde-saphyr 2.1、async-nats 0.51、tokio 1.54、redis 1.8、garde 特性收窄、Cargo.lock 27 包刷新 | `d66dc2f` |
| CI | 基建 action 对齐上游最新补丁版；4 个依赖 bot PR 本地取代并关闭 | `995bad5` / `6a0b285` |
| 文档 | 文档与代码一致性修复（10 处漂移） | `1dd2695` |
| 修复 | CodeQL `rust/cleartext-logging` 告警清零：测试诊断消息不再输出 `api_key` 长度 | `472b3ab` |
| 修复 | watcher 失败可观测测试改用独立临时目录（CI 宿主依赖） | `b2077f1` |

## 2. 发布资格门（五项机检，全部清零）

| 检查项 | 结果 |
| --- | --- |
| HEAD 上全部 workflow 成功 | ✅ CI success、CodeQL success、Release success（`feature-matrix.yml` 为每周定时 + 手动触发，不在 push 门禁内） |
| code scanning open 告警 = 0 | ✅ 0（告警 #173 随修复提交自动关闭） |
| dependabot open 告警 = 0 | ✅ 0（4 条 rustls 告警由 `fuzz/Cargo.lock` 升至 0.23.46 消除） |
| open 人类 PR = 0 | ✅ 0（6 个 bot PR 全部关闭并注明本地已取代） |
| open issue = 0 | ✅ 0 |

## 3. 测试与质量门禁

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --check` | ✅ 通过 |
| `cargo clippy --workspace --all-features --all-targets -- -D warnings` | ✅ 0 告警 |
| `cargo test --workspace --all-features` | ✅ **3266 通过 / 0 失败**（66 个测试二进制） |
| lib 单测（内联 2433 个） | ✅ 2425 通过、3 ignored、0 失败 |
| 集成测试（真实 NATS/Redis/etcd/Consul/Nacos） | ✅ 78 通过 / 0 跳过，无 mock 服务 |
| 行覆盖率（`cargo llvm-cov --workspace --all-features`） | ✅ **95.56%**（门禁 ≥90%，CI 实际执行 `--fail-under-lines 90`） |
| 示例编译 | ✅ `cargo build --examples --all-features` 退出码 0 |
| `cargo audit` | ✅ 388 依赖，0 漏洞，退出码 0 |
| `cargo deny check` | ✅ advisories / bans / licenses / sources 全 ok |
| `cargo check --workspace --all-features` | ✅ 0 error |
| `cargo check --workspace --no-default-features` | ✅ 0 error |
| `cargo publish --dry-run -p confers-macros` | ✅ 通过（根包 dry-run 因 `confers-macros` rc.13 尚未发布而无法解析，属预期；release.yml 已实现先发 macros 再发 root） |

## 4. 发布产物

| 产物 | 状态 |
| --- | --- |
| GitHub Release `v0.6.0-rc.13` | ✅ 已创建，`prerelease=true`（rc 不抢占 Latest），`draft=false` |
| crates.io `confers 0.6.0-rc.13` | ✅ 稀疏索引已可见，API HTTP 200 |
| crates.io `confers-macros 0.6.0-rc.13` | ✅ 由 release.yml `Publish confers-macros` 步骤发布 |
| Release workflow（`41848784509`） | ✅ Create GitHub Release + Publish to crates.io 均 success |

## 5. 版本传播点（13 个文件，全仓零残留）

`Cargo.toml`（workspace.package + `confers-macros` 钉子）、`Cargo.lock`、`examples/Cargo.toml`、`fuzz/Cargo.toml`、`fuzz/Cargo.lock`、`README.md`、`README_EN.md`、`docs/CHANGELOG.md`、`docs/USER_GUIDE.md`、`docs/FAQ.md`、`docs/PERFORMANCE.md`、`docs/TEST_SCENARIOS.md`、`docs/LIBRARY_INTEGRATION.md`、`docs/CONFIG_MACRO_GUIDE.md`。

校验：`grep -rn '0\.6\.0-rc\.12'`（排除 `target/`、`specmark/`、CHANGELOG 历史段）返回 0 行。

## 6. 过程中发现与处置

### 6.1 CI 首轮失败：watcher 测试依赖宿主 `/tmp` 可读性

- **现象**：`watcher::fs_watcher::tests::failed_watcher_reports_not_running_and_recv_returns_none` 在 Linux CI 上失败（`Test (default)` / `Test (full)` / `Test (recommended)` / `Coverage` 四个 job 同因失败），本地 macOS 通过。
- **根因**：该测试用 `std::env::temp_dir()` 建 watcher。R-watch-008 使 watch 建立同步化且递归，Linux CI 的 `/tmp` 含 root 所有的 `systemd-private-*` 子目录，递归 watch 报 `Permission denied (os error 13)`。测试前提「系统临时目录一定可 watch」是宿主属性假设，与被测契约无关。
- **处置**：改用独立 `tempfile::TempDir`，断言目标（失败可观测）不变 → `b2077f1`。
- **附带行为变更**：监视含不可读子目录的目录树现在会在构造时显式返回 `WatcherError`，而非静默成功。已写入 CHANGELOG（属「不再产出看似存活实则失聪的 watcher」的必然结果）。

### 6.2 CodeQL 告警修复

`tests/e2e/gap_sec_e2e.rs:401` 原 panic 文案输出 `cfg.api_key.len()`，被判定为敏感值派生信息外泄。改为不含占位符的固定文案，并给 `Probe` 加 `#[allow(dead_code)]`（与同文件第 321-322 行既有惯例一致）。断言语义不变：`Ok` 分支仍 panic、`Err` 分支仍校验错误文案含 decrypt/encrypt/key。

### 6.3 6 个 dependabot PR 处置

全部关闭而非合并，每个评论注明本地实际钉住的版本：

| PR | 包 | 本地实际 |
| --- | --- | --- |
| 65 | `rust-lang/crates-io-auth-action` | v1.0.6 |
| 64/63/62 | `github/codeql-action` | v4.39.2 |
| 61 | `Swatinem/rust-cache` | v2.9.3 |
| 60 | `rustls`（/fuzz） | 0.23.46 |

合并会使 SHA 回退到 PR 分支的旧提交，属负向变更。

## 7. 跳过与降级记录

| 项 | 说明 |
| --- | --- |
| `feature-matrix.yml` | 未在本次 push 上运行——该 workflow 为每周定时（周一 05:47）+ 手动触发，PR/push 门禁由 `ci.yml` 策展组合矩阵兜底。非跳过门禁。 |
| 提交前 3 路 subagent 审查 | 3 个并行 subagent（安全/架构/性能）均 600 秒无响应超时，属环境问题。已降级为：1 个独立 subagent 审查（结论：无凭据泄漏、断言未削弱、版本一致）+ 本人确定性核查（凭据扫描、断言强度、13 处版本一致）。**未获完整三路独立审查**。 |
| `cargo publish --dry-run`（根包） | 因 `confers-macros` rc.13 尚未发布而无法解析依赖。已单独验证 `-p confers-macros` 打包通过，并确认 release.yml 先发 macros 再发 root。 |
| `*.profraw` 残留 | 仓根 6 个覆盖率产物文件属历史遗留，不影响发布，列入 Deferred。 |
| 预检查清单复用 | `~/.cache/release-pipeline/` 下存在本会话早前生成的 18 阶段清单（10/18），本次为独立发布轮次，重新 `checklist.py init` 生成新清单。 |

## 8. 未完成项（均为不可修复项，附解锁前提）

| 项 | 类型 | 原因与路径 |
| --- | --- | --- |
| 下游 fluxgate 侧验证 R-watch-008 | 硬外部阻塞 | fluxgate 是独立仓库（规则 20 边界外）。解锁前提：fluxgate 将 `confers` 升级到 `0.6.0-rc.13` 并跑通热更新并发测试；届时若仍失败，需回到本仓复查而非在下游修。 |
| `*.profraw` 清理与 `.gitignore` 规则 | 仓库边界内但非本变更范围 | 已在 proposal 的 Deferred 记录。重开条件：下一次涉及仓根卫生的变更或用户主动要求。 |

## 9. specmark 变更追踪

- 变更目录：`specmark/changes/release-v0-6-0-rc13/`
- 产物：`proposal.md`、`design.md`、`tasks.md`（18 个任务，17 个已勾选）、`specs/release-versioning/spec.md`、`specs/security-scanning/spec.md`、`specs/dependency-governance/spec.md`
- 复杂度判定：`complex`（18 任务 / 17 模块）
- 待勾选项：T018（本报告），勾选后 18/18

---

## AI 使用声明

本报告的编制过程使用了 AI 助手（ZCode agent）。数据来源为本次会话中实际执行的命令输出：`git`、`cargo`（fmt/clippy/test/llvm-cov/audit/deny/check/publish --dry-run）、`gh`（run view / pr list / api）、`docker ps`、`curl`。所有量化结论均可由上述命令复现；未经命令验证的推断已显式标注为假设或跳过项。报告由 AI 起草，发布决策（版本号、dependabot 处置方式、告警修复方式）经用户明确确认。
