# 设计：修复审查问题实施方案

## 修复清单

### 1. CI workflow 组件修复

**文件**: `.github/workflows/ci.yml`

```yaml
# deny job - 添加 components
deny:
  name: Cargo Deny
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy  # 添加此行
    - uses:EmbarkStudios/cargo-deny-action@v1
      with:
        command: check advisories
```

### 2. Fuzz workflow 修复

**文件**: `.github/workflows/fuzz.yml`

使用 `taiki-e/install-action@cargo-fuzz` 替代手动安装：

```yaml
- name: Install cargo-fuzz
  uses: taiki-e/install-action@cargo-fuzz
```

### 3. 格式化修复

运行命令：
```bash
cargo fmt --all
```

### 4. Clippy 警告修复

运行命令：
```bash
cargo clippy --all-features --fix --allow-dirty
```

主要警告类型：
- 冗余闭包 (redundant_closure)
- 未使用变量

## 实施顺序

1. 修复 CI workflow
2. 修复 fuzz workflow
3. 格式化代码
4. 修复 Clippy 警告
5. 验证
