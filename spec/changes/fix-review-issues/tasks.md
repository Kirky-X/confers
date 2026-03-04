# 任务列表：修复审查问题

## Task 1: 修复 CI workflow deny job

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: 编辑 deny job**

```yaml
deny:
  name: Cargo Deny
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
    - uses:EmbarkStudios/cargo-deny-action@v1
      with:
        command: check advisories
```

**Step 2: 验证 YAML 语法**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

**Step 3: 提交**

```bash
git add .github/workflows/ci.yml
git commit -m "fix(ci): add clippy component to deny job"
```

---

## Task 2: 修复 fuzz workflow

**Files:**
- Modify: `.github/workflows/fuzz.yml`

**Step 1: 编辑安装步骤**

```yaml
- name: Install cargo-fuzz
  uses: taiki-e/install-action@cargo-fuzz
```

**Step 2: 提交**

```bash
git add .github/workflows/fuzz.yml
git commit -f "fix(ci): use taiki-e/install-action for cargo-fuzz"
```

---

## Task 3: 格式化代码

**Step 1: 运行格式化**

```bash
cargo fmt --all
```

**Step 2: 验证**

```bash
cargo fmt --all -- --check
```

**Step 3: 提交**

```bash
git add -A
git commit -m "style: fix formatting issues"
```

---

## Task 4: 修复 Clippy 警告

**Step 1: 自动修复**

```bash
cargo clippy --all-features --fix --allow-dirty
```

**Step 2: 手动检查**

检查剩余警告，确保无重要问题。

**Step 3: 验证**

```bash
cargo clippy --all-features 2>&1 | grep -c warning
# 应该输出 0
```

**Step 4: 提交**

```bash
git add -A
git commit -m "fix: resolve clippy warnings"
```

---

## Task 5: 最终验证

**Step 1: 编译检查**

```bash
cargo check --all-features
```

**Step 2: 测试**

```bash
cargo test --all-features
```

**Step 3: 提交**

```bash
git add -A
git commit -m "chore: final fixes"
```
