# ADR-031: DynamicField

## 状态
Accepted

## 背景
需要字段级动态配置句柄。

## 决策
实现 DynamicField<T>，使用 arc-swap 提供无锁读取。

## 后果
- 正面：字段级精度
- 负面：内存开销

---

# ADR-032: Context-Aware

## 状态
Accepted

## 背景
需要按运行时上下文返回不同配置值。

## 决策
ContextAwareField<T> + EvaluationContext。

## 后果
- 正面：支持多租户、灰度发布

---

# ADR-033: 配置快照

## 状态
Accepted

## 背景
需要配置快照持久化。

## 决策
SnapshotManager 带时间戳快照，敏感字段脱敏。

## 后果
- 正面：问题回溯

---

# ADR-034: Config Groups

## 状态
Accepted

## 背景
需要可组合配置模块。

## 决策
ModuleRegistry 支持配置组切换。

## 后果
- 正面：配置复用

---

# ADR-035: ConfigBus

## 状态
Accepted

## 背景
需要多实例配置同步。

## 决策
ConfigBus trait + NATS/Redis 实现。

## 后果
- 正面：分布式同步

---

# ADR-036: Progressive Reload

## 状态
Accepted

## 背景
需要分阶段热重载。

## 决策
- Immediate
- Canary（健康检查）
- Linear（概率切换）

## 后果
- 正面：变更安全

---

# ADR-037: 轮询源

## 状态
Accepted

## 背景
需要统一轮询抽象。

## 决策
PolledSource trait + ETag 条件请求。

## 后果
- 正面：支持 HTTP 轮询

---

# ADR-038: _FILE 后缀

## 状态
Accepted

## 背景
需要支持 Docker/K8s Secrets。

## 决策
环境变量 _FILE 后缀读取文件内容。

## 后果
- 正面：容器集成

---

# ADR-039: CLI 诊断工具

## 状态
Accepted

## 背景
需要 CLI 可观测性工具。

## 决策
confers-cli: inspect/validate/export/diff/snapshot。

## 后果
- 正面：调试便捷

---

# ADR-040: 值溯源精确化

## 状态
Accepted

## 背景
需要精确的值溯源。

## 决策
SourceLocation 行列位置。

## 后果
- 正面：精确错误定位
