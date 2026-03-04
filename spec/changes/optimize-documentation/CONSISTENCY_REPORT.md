# 文档与代码一致性报告

## 更新时间
2026-03-04

---

## 执行摘要

文档中提到的 features 与 Cargo.toml 中的实际定义存在**不一致**。

| 类型 | 数量 |
|------|------|
| 文档有，代码没有 | 3 |
| 代码有，文档没有 | 22+ |

---

## 详细差异

### 文档有，代码没有 (需删除)

| Feature | 位置 | 问题 |
|---------|------|------|
| `derive` | API_REFERENCE.md:92 | `derive` 不是独立 feature，由 `confers-macros` 自动提供 |
| `monitoring` | API_REFERENCE.md:99 | Cargo.toml 中不存在此 feature |
| `hocon` | API_REFERENCE.md:102 | Cargo.toml 中不存在此 feature |

### 代码有，文档没有 (需添加)

#### 核心缺失
| Feature | 类型 | 优先级 |
|---------|------|--------|
| `typescript-schema` | TypeScript 类型生成 | 高 |
| `security` | 安全模块 | 高 |
| `key` | 密钥管理 | 高 |
| `tracing` | 分布式追踪 | 中 |

#### 格式支持缺失
| Feature | 描述 |
|---------|------|
| `yaml` | YAML 格式支持 |
| `ini` | INI 格式支持 |

#### 高级功能缺失
| Feature | 描述 |
|---------|------|
| `parallel` | 并行验证 |
| `progressive-reload` | 渐进式部署 |
| `interpolation` | 变量插值 |
| `hot-reload` | 热重载 (watch 别名) |

#### 远程源缺失
| Feature | 描述 |
|---------|------|
| `etcd` | Etcd 集成 |
| `consul` | Consul 集成 |
| `cache-redis` | Redis 缓存 |

#### 其他缺失
| Feature | 描述 |
|---------|------|
| `config-bus` | 配置事件总线 |
| `context-aware` | 上下文感知配置 |
| `modules` | 模块化配置 |
| `dynamic` | 动态字段 |
| `audit` | 审计日志 |
| `metrics` | 指标收集 |
| `migration` | 配置迁移 |
| `snapshot` | 快照回滚 |
| `profile` | 环境配置 |

---

## 预设特性对比

### 文档中的预设
| 预设 | 文档描述 |
|------|----------|
| `minimal` | `derive` |
| `recommended` | `derive`, `validation` |
| `dev` | `derive`, `validation`, `cli`, `schema`, `audit`, `monitoring` |
| `production` | `derive`, `validation`, `watch`, `encryption`, `remote`, `monitoring` |
| `full` | 所有特性 |

### Cargo.toml 中的预设
| 预设 | 实际定义 |
|------|----------|
| `minimal` | `env` |
| `recommended` | `toml`, `env`, `validation` |
| `dev` | `toml`, `json`, `yaml`, `env`, `cli`, `validation`, `schema`, `audit`, `profile`, `watch`, `migration`, `snapshot`, `dynamic` |
| `production` | `toml`, `env`, `watch`, `encryption`, `validation`, `audit`, `profile`, `metrics`, `schema`, `cli`, `migration`, `dynamic`, `progressive-reload`, `snapshot` |
| `full` | 所有 30+ 特性 |
| `distributed` | `toml`, `env`, `watch`, `validation`, `config-bus`, `progressive-reload`, `metrics`, `audit` |

### 预设差异
- 所有预设都错误地包含了 `derive`
- `dev`, `production`, `full` 预设包含了不存在的 `monitoring`
- 缺少 `distributed` 预设

---

## 修复优先级

### P0 - 必须修复
1. 删除 `derive` 引用（不是独立 feature）
2. 删除 `monitoring` 引用（不存在）
3. 删除 `hocon` 引用（不存在）
4. 添加 `typescript-schema`, `security`, `key` 文档

### P1 - 重要
5. 添加 `yaml`, `ini` 格式支持文档
6. 修正所有预设特性列表

### P2 - 改进
7. 添加其他 18+ 缺失 features 的文档

---

## 下一步行动

1. ✅ Task D.1: 提取 Cargo.toml Features (已完成)
2. ✅ Task D.2: 对比文档与代码 (进行中)
3. ⏳ Task D.3: 修复代码文档警告
4. ⏳ Task B.1: 创建 CHANGELOG 0.3.0
5. ⏳ Task A.x: 补充新功能文档
6. ⏳ Task C.x: 修正 API 文档准确性
