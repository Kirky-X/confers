# 提案：修复代码审查发现的问题

## 摘要

修复代码审查中发现的问题，包括 CI 配置、格式化和 Clippy 警告。

## 背景

在代码审查中发现以下问题需要修复：

1. **HIGH-001**: CI workflow 缺少 rust-toolchain 组件
2. **MED-001**: Clippy 警告未修复 (26个)
3. **MED-002**: fuzz workflow 缺少 cargo-fuzz 安装
4. **LOW-001**: 格式化不一致

## 目标

1. 修复 CI deny job 缺少的组件
2. 修复所有 Clippy 警告
3. 修复 fuzz workflow 安装步骤
4. 统一代码格式化

## 非目标

- 不修改核心功能代码
- 不添加新功能

## 范围

- `.github/workflows/ci.yml`
- `.github/workflows/fuzz.yml`
- `src/` 中的 Clippy 警告
- `benches/` 格式化
- `examples/` 格式化
- `confers-cli/` 格式化

## 成功标准

1. `cargo fmt --all -- --check` 通过
2. `cargo clippy --all-features` 无警告
3. CI workflow 配置正确

## 风险

无重大风险，均为简单修复。
