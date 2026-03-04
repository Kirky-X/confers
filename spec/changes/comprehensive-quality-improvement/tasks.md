# 任务列表：全面质量提升

## Task 1: 修复 confers-cli Clippy 警告

**Files:**
- Modify: `confers-cli/src/**/*.rs`

**Step 1: 诊断警告**

```bash
cargo clippy -p confers-cli --all-features 2>&1 | grep "warning:"
```

**Step 2: 自动修复**

```bash
cargo clippy -p confers-cli --fix --allow-dirty
```

**Step 3: 检查剩余警告**

```bash
cargo clippy -p confers-cli --all-features
```

期望：0 warnings

**Step 4: 验证编译和测试**

```bash
cargo build -p confers-cli
cargo test -p confers-cli
```

**Step 5: 提交**

```bash
git add confers-cli/
git commit -m "fix(cli): resolve all clippy warnings in confers-cli"
```

---

## Task 2: 修复 Examples - full_stack 警告

**Files:**
- Modify: `examples/src/full_stack.rs`

**Step 1: 诊断警告**

```bash
cargo clippy --bin full_stack -p confers-examples --all-features 2>&1 | grep "warning:"
```

**Step 2: 自动修复**

```bash
cargo clippy --bin full_stack -p confers-examples --fix --allow-dirty
```

**Step 3: 验证**

```bash
cargo build --bin full_stack -p confers-examples
```

**Step 4: 提交**

```bash
git add examples/
git commit -m "fix(examples): resolve clippy warnings in full_stack"
```

---

## Task 3: 修复 Examples - key_rotation 警告

**Files:**
- Modify: `examples/src/key_rotation.rs`

**Step 1: 自动修复**

```bash
cargo clippy --bin key_rotation -p confers-examples --fix --allow-dirty
```

**Step 2: 验证**

```bash
cargo build --bin key_rotation -p confers-examples
```

**Step 3: 提交**

```bash
git add examples/
git commit -m "fix(examples): resolve clippy warnings in key_rotation"
```

---

## Task 4: 修复 Examples - migration 警告

**Files:**
- Modify: `examples/src/migration.rs`

**Step 1: 自动修复**

```bash
cargo clippy --bin migration -p confers-examples --fix --allow-dirty
```

**Step 2: 验证**

```bash
cargo build --bin migration -p confers-examples
```

**Step 3: 提交**

```bash
git add examples/
git commit -m "fix(examples): resolve clippy warnings in migration"
```

---

## Task 5: 修复 Examples - progressive_reload 警告

**Files:**
- Modify: `examples/src/progressive_reload.rs`

**Step 1: 自动修复**

```bash
cargo clippy --bin progressive_reload -p confers-examples --fix --allow-dirty
```

**Step 2: 验证**

```bash
cargo build --bin progressive_reload -p confers-examples
```

**Step 3: 提交**

```bash
git add examples/
git commit -m "fix(examples): resolve clippy warnings in progressive_reload"
```

---

## Task 6: 修复 Examples - remote_consul 警告

**Files:**
- Modify: `examples/src/remote_consul.rs`

**Step 1: 自动修复**

```bash
cargo clippy --bin remote_consul -p confers-examples --fix --allow-dirty
```

**Step 2: 验证**

```bash
cargo build --bin remote_consul -p confers-examples
```

**Step 3: 提交**

```bash
git add examples/
git commit -m "fix(examples): resolve clippy warnings in remote_consul"
```

---

## Task 7: 修复 Examples - config_groups 警告

**Files:**
- Modify: `examples/src/config_groups.rs`

**Step 1: 自动修复**

```bash
cargo clippy --bin config_groups -p confers-examples --fix --allow-dirty
```

**Step 2: 验证**

```bash
cargo build --bin config_groups -p confers-examples
```

**Step 3: 提交**

```bash
git add examples/
git commit -m "fix(examples): resolve clippy warnings in config_groups"
```

---

## Task 8: 添加 confers-cli 测试

**Files:**
- Create: `confers-cli/tests/test_cli.rs`

**Step 1: 创建测试文件**

```rust
// confers-cli/tests/test_cli.rs

use std::process::Command;

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .current_dir("confers-cli")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("confers") || stdout.contains("Usage"));
}

#[test]
fn test_version_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .current_dir("confers-cli")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.3.0"));
}
```

**Step 2: 运行测试验证**

```bash
cargo test -p confers-cli
```

期望：新增测试通过

**Step 3: 提交**

```bash
git add confers-cli/tests/
git commit -m "test(cli): add basic CLI tests"
```

---

## Task 9: 最终验证

**Step 1: 完整 Clippy 检查**

```bash
cargo clippy --workspace --all-features 2>&1 | grep -c "warning"
```

期望：0

**Step 2: 完整测试套件**

```bash
cargo test --workspace --all-features
```

期望：全部通过

**Step 3: 示例编译验证**

```bash
cargo build --examples --all-features
```

期望：全部成功

**Step 4: 提交总结**

```bash
git add -A
git commit -m "chore: complete comprehensive quality improvement

- Fixed all clippy warnings in confers-cli (5)
- Fixed all clippy warnings in examples (34)
- Added basic CLI tests
- All tests passing, 0 warnings"
```

---

## 任务状态

- [ ] Task 1: 修复 confers-cli Clippy 警告
- [ ] Task 2: 修复 full_stack 警告
- [ ] Task 3: 修复 key_rotation 警告
- [ ] Task 4: 修复 migration 警告
- [ ] Task 5: 修复 progressive_reload 警告
- [ ] Task 6: 修复 remote_consul 警告
- [ ] Task 7: 修复 config_groups 警告
- [ ] Task 8: 添加 confers-cli 测试
- [ ] Task 9: 最终验证
