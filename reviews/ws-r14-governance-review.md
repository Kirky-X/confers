# ws-R14 治理复核记录 — confers（T039）

> Generated: 2026-10-01 · 依据: `FEATURE_AUDIT_REPORT.md` §9.2-5（workspace 根，只读审计源）
> 审计原文: "confers：security-rules 与 encryption 解耦；三个近义 feature 对的别名迁移。"
> 复核范围: 两项逐条"成立则做（测试全绿），过时则记录"，不预置结论

## 0. 结论总表

| # | 审计项 | 结论 | 证据位置 |
|---|---|---|---|
| 1 | security-rules 与 encryption 解耦 | **成立，已解耦**（`security` 改为自带轻量原语、不再隐含 `encryption`；有效公开面不变） | `Cargo.toml:147-151`、`src/security/mod.rs:893-922`、`tests/e2e/presets_e2e.rs:246-254`（prs08 负面断言） |
| 2 | 三个近义 feature 对的别名迁移 | **已由 T050 关闭**（45ce726 落地 + bc15912 收尾）；本次修正其文档残留（README×2 预设表、TEST_SCENARIOS §3.1、CONTRIBUTING） | 见 §2 逐项留证 |

## 1. security-rules 与 encryption 解耦 — 成立，已解耦

**解耦前状态（耦合实证）**：

- `Cargo.toml` 原定义：`security = ["encryption", "dep:hex"]`、`security-rules = ["security", "dep:ipnet"]`——`security-rules` 经两级链式传递隐含 `encryption` 全栈。
- 红态实测（改动前）：`cargo tree --no-default-features --features security-rules` 依赖图含 `chacha20poly1305 v0.11.0`、`hkdf v0.13.0`、`tokio v1.53.1`、`async-trait`、`getrandom v0.4.3`——一个纯计算校验库拉入完整加密与异步运行时栈。
- 不存在任务书预判的 `cfg(any(security-rules, encryption))` 类联合门控；唯一联合门是 `src/lib.rs:95` 的 `cfg(any(remote, security-rules))`（`ip_blocklist`，与 encryption 无关）。耦合完全在 feature 声明层。

**耦合成因分析（为何当初挂上 encryption）**：

- 规则引擎本体（`src/security/rules/`：cors/jwt/ssrf/tls/remap）外部依赖仅 `ipnet`（经 `src/ip_blocklist.rs`，其 import 面 `ipnet/std/LazyLock`）与核心 `ConfigProvider`/`AnnotatedValue`；模块文档自述 "synchronous, pure-computation checks ... do not perform any I/O operations"（`src/security/rules/mod.rs:66-71`）。**规则引擎不需要任何加密原语。**
- `security` 中间层模块的完整外部依赖面：`regex`（无条件）、`serde`（无条件）、`zeroize`（`secure_string.rs:33`，SecureString 零化）、`sha2` + `hex`（`secure_string.rs:264-274`，`fingerprint()` 运行时指纹）。`encryption` 栈中的 chacha20poly1305/hkdf/tokio/async-trait/getrandom **无一被 security 模块引用**——耦合面比审计预估的更小，解耦成立且代价可控。

**修复内容**：

1. `Cargo.toml:147-151`：`security = ["dep:zeroize", "dep:sha2", "dep:hex"]`（剥离 `encryption`，三个实际原语改为自身声明）；`security-rules = ["security", "dep:ipnet"]` 不变，经此同享解耦。
2. `src/security/mod.rs:893-922`：`secure_string` 模块声明与 `error_sanitization`/`secure_string` 两处 pub re-export 的内层 `#[cfg(feature = "encryption")]` 删除，收敛为随模块存在而编译。**语义等价论证**：整个 security 模块挂在 `src/lib.rs:144` 的 `cfg(feature = "security")` 下，原链式语义下 security⇒encryption 恒成立，故这三处内层 encryption 门控恒真——删除不改变任何组合下的有效公开面；`--features encryption` 单开组合行为亦不变（security 模块本就不编译，实证见 §3 回归）。`config_injector`/`input_validation` 的 `security-rules` 门控保留（security 单开不含 rules 的语义继续成立）。
3. TDD 门禁：`tests/e2e/presets_e2e.rs` prs08 原以 `("security", &["encryption"])` 正向固化耦合；先改为解耦负面断言（`security` 成员不得含 `encryption`，:246-254）——红（`0 passed; 1 failed`）→ 落地 Cargo.toml/mod.rs → 绿（`4 passed`）。

**量化收益（FAQ 口径：`cargo tree -e normal --no-default-features --features <组合>`，直接依赖=深度 1 去重，传递=全节点按包名去重）**：

| 组合 | 改动前 | 改动后 |
|---|---|---|
| `recommended` | 27 直接 / 131 传递（docs/FAQ.md 原记载，同 Cargo.lock 实测） | **22 直接 / 119 传递**（FAQ 表已同步更新） |
| `security-rules` 单开 | 含 chacha20poly1305/hkdf/tokio/async-trait/getrandom | 上述加密/异步栈 0 命中，104 传递去重 |

## 2. 三个近义 feature 对别名迁移 — 已由 T050 关闭，文档残留本次修正

**落地实证（迁移到哪个正名、旧名保留与提示、文档/CI 同步，逐项核对）**：

| 项 | 正名（持有依赖） | 兼容别名（下一版移除） | 证据 |
|---|---|---|---|
| .env 加载 | `dotenv = ["dep:dotenvy"]` | `env = ["dotenv"]`（Cargo.toml:136-137） | 45ce726 diff：`default`/六预设/`[[test]] key_e2e` 全部记名正名 |
| 密钥管理 | `key-management = ["encryption", "dep:chrono", "dep:rand", "dep:hex"]` | `key = ["key-management"]`（Cargo.toml:153-154） | 同上 |
| Schema 生成 | `json-schema = ["dep:schemars"]` | `schema = ["json-schema"]`（Cargo.toml:159-160） | `typescript-schema`/`cli` 链改指正名 |

- **deprecated 提示**：`build.rs:14-17` `DEPRECATED_ALIASES` 表将三个别名各映射到正名，启用任一别名输出编译期 `cargo:warning`（"will be removed in the next release"）；rerun-if-changed 收窄由审查收尾 bc15912 补齐。满足「旧名保留一个版本的 deprecated 提示」要求。
- **正名断言固化**：`tests/e2e/presets_e2e.rs` prs08 别名链断言（:272-274 `("env", &["dotenv"])` 等三条）+ 六预设正名展开断言；提交信息记载 Red→Green（presets_e2e 正名断言 3 failed→4 passed）。
- **文档同步**：docs/CHANGELOG.md:22-25 迁移专节（正名语义理由、别名机制、文档同步清单、doctor 测试组合冲突修复）；README/README_EN 功能矩阵保留别名行并标注弃用（README.md:173/181/193）。
- **CI 同步**：ci.yml 测试矩阵（default/recommended/full）全部经正名预设生效，无旧名引用；scripts/ 无 feature 组合脚本。

**T050 残留清单（本次复核发现并全部修正）**：

1. README.md:98/143/153-158 与 README_EN.md:98/143/153-158——默认特性 prose 与功能预设表 6 行仍列 `env`、2 行（dev/production）仍列 `schema`（矩阵节已同步、预设节漏改）。
2. docs/TEST_SCENARIOS.md §3.1 依赖链表——`key`/`typescript-schema → schema`/`dotenv → env`/`default → toml, json, env` 四处停在迁移前状态；PRS-08 行（:608）仍写 "security→encryption"。
3. docs/CONTRIBUTING.md:100——默认特性 `toml`、`json`、`env`。

以上已随本次解耦一并改为正名（CHANGELOG Unreleased 解耦节第三条注明"残留旧名一并修正"）。

**别名移除到期项**（下一版本 0.6.0-rc.7 或 0.7.0 执行，此处留档防遗忘）：删除 `Cargo.toml` 三条别名、`build.rs` `DEPRECATED_ALIASES` 三表项、README/README_EN 功能矩阵三行别名说明、prs08 三条别名链断言。

## 3. 验证基线

- `cargo fmt --check` 通过；`cargo clippy --workspace --all-targets --quiet` 零告警
- `cargo test --workspace --quiet`：exit=0，62 个 test target 全 ok（default / recommended / full 三组合同结果，CI 矩阵同口径）
- 新合法组合回归：`--no-default-features --features security` / `--features security-rules` / `--features encryption` 单开，各 62 target 全 ok（encryption 单开验证收敛后行为不变）
- prs08：红（改断言后 `1 failed`）→ 绿（落地后 `4 passed; 0 failed`）
- `cargo tree` 断言：`--no-default-features --features security-rules` 图中 chacha20poly1305/hkdf/tokio/async-trait/getrandom 由红态全部在图 → 绿态 0 命中

## 4. 附带核对与附带发现

- **ARCHITECTURE.md:120/131 无需改**：特性门控模块表首列是模块名（`key`=src/key/、`schema`=src/schema/）而非 feature 名，门控列已是正名——核对结论留档，防后续误报。
- **FAQ 依赖数量表其余行不受解耦影响**：minimal/dev/cli 不含 security-rules；production/full 显式含 encryption——仅 `recommended` 行需且已更新。
- **examples crate 恒以 `features = ["full"]` 依赖 confers**（examples/Cargo.toml:115），无法在示例层覆盖瘦身组合回归——低优先级观察项，记录不修（瘦身组合由 presets_e2e + 本节三单开组合覆盖）。
- **`.gitignore:79` 整目录忽略 `reviews/` 与跨仓惯例相悖（治理分歧留档）**：inklog 同位置采用白名单模式（`reviews/*` + `!reviews/**/*.md`，报告类 md 全部入库），且本仓 `tests/e2e/presets_e2e.rs:16-17` 引用的 `reviews/acceptance-report.md` 实为未跟踪本地文件（引用悬空）。本记录按任务授权以 `git add -f` 强制入库成忽略例外；是否对齐 inklog 白名单模式并补 tracking acceptance-report.md 属独立治理项，待立项。
