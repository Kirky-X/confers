# Git 提交核心功能

提供智能的 Git 提交功能，包括上下文感知、自动生成 commit message 等。

## 核心函数

### 1. analyzeChanges

分析变更内容，识别变更类型和影响范围。

```typescript
interface ChangeAnalysis {
  type: 'feat' | 'fix' | 'refactor' | 'docs' | 'style' | 'test' | 'chore';
  scope?: string;
  files: string[];
  description: string;
}

function analyzeChanges(filePatterns: string[]): ChangeAnalysis[]
```

**实现逻辑：**
1. 获取变更文件的 diff
2. 分析文件路径和变更内容
3. 识别变更类型（feat/fix/refactor 等）
4. 生成变更描述

### 2. generateCommitMessage

根据变更分析生成规范的 commit message。

```typescript
function generateCommitMessage(analysis: ChangeAnalysis): string
```

**生成规则：**
- `feat`: `feat(<scope>): <描述>`
- `fix`: `fix(<scope>): <描述>`
- `refactor`: `refactor(<scope>): <描述>`
- `docs`: `docs: <描述>`
- `style`: `style: <描述>`
- `test`: `test: <描述>`
- `chore`: `chore: <描述>`

### 3. smartStage

智能暂存相关文件。

```typescript
async function smartStage(filePatterns: string[]): Promise<string[]>
```

**策略：**
1. 分析会话上下文中修改的文件
2. 识别核心变更文件和关联文件
3. 仅暂存相关文件
4. 返回已暂存的文件列表

### 4. commit

执行提交。

```typescript
async function commit(message: string, options?: {
  amend?: boolean;
  noVerify?: boolean;
}): Promise<string>
```

### 5. executeSmartCommit

执行智能提交（默认模式）。

```typescript
async function executeSmartCommit(options?: {
  amend?: boolean;
  message?: string;
  all?: boolean;
  noVerify?: boolean;
}): Promise<{
  success: boolean;
  commitHash?: string;
  message: string;
  stagedFiles: string[];
}>
```

**工作流程：**
1. 获取 git status 和 diff
2. 分析变更类型
3. 生成 commit message（如果未指定）
4. 暂存相关文件
5. 执行提交
6. 返回结果

## 使用示例

```typescript
// 智能提交（默认）
const result = await executeSmartCommit();

// 全量提交
const result = await executeSmartCommit({ all: true });

// 修改最后一次提交
const result = await executeSmartCommit({ amend: true });

// 自定义提交信息
const result = await executeSmartCommit({ 
  message: "fix: 修复登录bug" 
});
```

## 文件分析规则

| 文件模式 | 变更类型 | 默认 scope |
|----------|----------|------------|
| `src/**/*.rs` | `feat`, `fix`, `refactor` | 模块名 |
| `docs/**/*.md` | `docs` | - |
| `tests/**/*.rs` | `test` | - |
| `Cargo.toml` | `chore` | deps |
| `.github/**` | `chore` | ci |

## 变更检测

```bash
# 获取变更文件
git status --porcelain

# 获取变更 diff
git diff --staged
git diff

# 获取最后一次提交
git show HEAD --stat
```

## 验证规则

1. **文件数量检查**：单次提交不宜超过 10 个文件
2. **变更类型检查**：确保所有变更属于同一类型
3. **message 规范检查**：符合 Conventional Commits 格式
4. **暂存区检查**：确保暂存了相关文件
