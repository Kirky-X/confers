# /commit

智能提交代码，自动生成规范的 commit message。

## 使用方法

```
/commit [--amend] [--message "msg"] [--no-verify] [--all]
```

## 参数

- `--amend` - 修改最后一次提交（未推送时）
- `--message "msg"` - 自定义提交信息
- `--no-verify` - 跳过 pre-commit 检查（不推荐）
- `--all` - 提交所有变更（默认行为：仅提交上下文相关文件）

## 示例

```
/commit                    # 智能分析，仅提交上下文修改的文件
/commit --all             # 提交所有变更
/commit --amend           # 修改最后一次提交
/commit --message "fix: 修复登录bug"  # 使用自定义信息
```

## 功能

- **上下文感知**：自动识别会话中修改的文件，仅提交相关变更
- **智能暂存**：分析变更，选择性暂存相关文件
- **自动生成 message**：根据变更类型和内容生成规范的 commit message
- **原子性提交**：确保每次提交只包含一个逻辑变更
- **提交前验证**：检查暂存内容，避免提交不相关代码

## Commit Message 格式

遵循 Conventional Commits 规范：

- `feat:` 新功能
- `fix:` 修复 bug
- `refactor:` 重构代码
- `docs:` 文档更新
- `style:` 代码格式调整
- `test:` 测试相关
- `chore:` 构建/工具相关

## 工作流程

### 智能模式（默认）

1. 分析会话上下文中修改的文件
2. 识别相关变更文件
3. 仅暂存和提交相关文件
4. 生成规范的 commit message
5. 验证并提交

### 全量模式（--all）

1. 获取所有变更文件（`git status`）
2. 暂存所有变更
3. 分析变更类型生成 message
4. 提交所有文件
