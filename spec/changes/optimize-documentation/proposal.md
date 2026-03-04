# Proposal: 文档内容优化

## Summary

根据实际代码情况，对 `docs/*.md` 文档进行全面优化，确保文档与 v0.3.0 版本的代码实现保持一致。

---

## Motivation

当前项目存在严重的**文档与代码不一致**问题：

### 版本差异
- **当前代码版本**: 0.3.0
- **文档记录版本**: 0.2.2 (2026-01-25)
- **缺失**: 0.3.0 版本的 CHANGELOG 条目

### 新功能无文档
从 `migrate-from-target-project` 分支合并后，新增了 **6,855 行代码**，但文档中完全没有记录：

| 模块 | 代码量 | 文档状态 |
|------|--------|----------|
| `src/key/` | ~2,000 行 | ❌ 无文档 |
| `src/security/` | ~3,600 行 | ❌ 无文档 |
| `src/schema/` | ~300 行 | ❌ 无文档 |
| `confers-cli/` | ~800 行 | ❌ 无文档 |

### 特性列表不一致
| 问题 | 示例 |
|------|------|
| 文档有，代码没有 | `monitoring`, `hocon`, `derive` |
| 代码有，文档没有 | `typescript-schema`, `security`, `key`, `tracing` |
| 预设不匹配 | `dev`, `production`, `full` 与实际 Cargo.toml 定义不符 |

### 文档质量问题
- 9 个 `cargo doc` 警告未修复
- 代码示例未经验证
- 交叉引用链接损坏

**如果不解决**，用户将：
- 无法使用新功能（没有文档说明）
- 被误导使用不存在的功能
- 在升级时遇到困惑（版本不匹配）

---

## Goals

1. **整体一致性检查** - 建立 Cargo.toml features 与文档的完整映射
2. **版本同步更新** - 创建 CHANGELOG 0.3.0 条目，记录所有变更
3. **补充新功能文档** - 为 key、security、typescript-schema 模块编写完整文档
4. **API 文档修正** - 修正特性列表和预设，确保与代码 100% 一致

---

## Non-Goals

- **不改变代码实现** - 只修改文档，不改变源代码
- **不创建新的文档结构** - 在现有文档框架内补充内容
- **不翻译文档** - 保持中文文档，不添加英文版本
- **不重写现有正确内容** - 只修正错误和补充缺失

---

## Scope

### 涉及的文件

**修改的文件：**
- `docs/CHANGELOG.md` - 添加 0.3.0 条目
- `docs/API_REFERENCE.md` - 更新特性列表，新增 API 章节
- `docs/USER_GUIDE.md` - 添加新功能使用示例
- `docs/CONFIG_MACRO_GUIDE.md` - 更新宏文档

**修复的代码文件：**
- `src/merger/strategy.rs` - 修复文档链接警告
- `src/dynamic.rs` - 修复 HTML 标签警告

**可选新增：**
- `docs/KEY_MANAGEMENT.md` - 密钥管理独立指南
- `docs/SECURITY_GUIDE.md` - 安全配置独立指南

### 不涉及的文件

- `src/**/*.rs` - 不修改源代码逻辑（仅修复文档注释）
- `confers-macros/**` - 不修改宏实现
- `examples/**` - 不修改示例代码
- `README.md` / `README_zh.md` - 不修改主页（已优化）

---

## Success Criteria

### 必须达成

- [ ] `CHANGELOG.md` 包含 0.3.0 版本条目
- [ ] `cargo doc --all-features` 无警告
- [ ] `cargo test --doc` 全部通过
- [ ] 所有新功能（key, security, typescript-schema）有 API 文档
- [ ] 特性列表与 Cargo.toml 100% 一致
- [ ] 所有预设特性定义准确

### 可选达成

- [ ] 创建独立的密钥管理指南
- [ ] 创建独立的安全配置指南
- [ ] 添加更多使用示例

---

## Risks

| 风险 | 级别 | 缓解措施 |
|------|------|----------|
| 文档内容不准确导致用户误解 | 中 | 每阶段运行 `cargo test --doc` 验证 |
| 工作量超出预期 | 低 | 按阶段执行，每阶段独立可交付 |
| 遗漏某些新 API | 中 | 自动扫描 `pub use` 语句 |
| 文档格式不一致 | 低 | 使用统一的 markdown 模板 |

---

## Dependencies

- 无外部依赖
- 基于 commit `6c88ad8` (merge migrate-from-target-project)
- 依赖 `spec/changes/optimize-documentation/design.md`

---

## Timeline

| 阶段 | 时间 | 输出 |
|------|------|------|
| D - 整体一致性检查 | 2-3 小时 | Feature 矩阵、一致性报告 |
| B - 版本同步更新 | 1-2 小时 | CHANGELOG 0.3.0 |
| A - 补充新功能文档 | 4-6 小时 | API 文档、使用示例 |
| C - API 文档修正 | 2-3 小时 | 准确的特性列表 |
| **总计** | **9-14 小时** | 完整、准确的文档 |

---

## Related Changes

- 依赖: `migrate-from-target-project` (已完成)
- 相关: `comprehensive-quality-improvement` (已完成)
- 相关: `fix-clippy-warnings` (已完成)
