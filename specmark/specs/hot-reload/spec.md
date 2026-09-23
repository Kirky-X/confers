# Spec — hot-reload

> Main spec for capability `hot-reload`.

## Requirements

### R-watch-001: 原子替换后监听存活

单文件 FsWatcher 在 rename 原子替换（编辑器写盘）或删除重建后继续收到后续变更事件。
**验收标准：**
- 连续两次 `rename(tmp, path)` 均各收到恰好变更事件（回归测试钉住）
- 删除后重建路径的同名文件变更仍可观测

### R-watch-002: 事件丢弃可观测

通道满丢弃事件时产生 tracing warn 且丢弃计数可查询。
**验收标准：**
- `dropped_events()` 在模拟慢消费者后 > 0 且 warn 日志发出

### R-watch-003: 回调 panic 不毒化

DynamicField::update 的用户回调 panic 被隔离；store 与其余回调不受影响。
**验收标准：**
- panic 回调后 update 正常返回、其余回调执行、计数可见

### R-watch-004: dynamic handle 单例

宏生成 `*_handle()` 多次调用返回共享实例。
**验收标准：**
- 两次调用 Arc ptr_eq；经 A handle update 后 B handle 可见

### R-watch-005: 并发 update 观测一致

并发 update 下回调观测顺序与最终存储值一致（过期回调跳过）。
**验收标准：**
- 多线程并发 update 测试最后观测值 == 最终 store 值

### R-watch-006: ChangeStream Lagged 显式

订阅者落后被 FIFO 驱逐后收到显式 Lagged/resync 信号而非静默跳过。
**验收标准：**
- 驱逐后 `get(version)` 返回 Lagged{from}；流消费端收到 resync 事件

### R-watch-007: AdaptiveDebouncer 接线

FsWatcher 事件出口经 AdaptiveDebouncer 合并；WAT-18 回归测试进入常规 CI。
**验收标准：**
- 快速连写 N 次合并为 ≤ 去抖窗口内有限次事件且不丢最后值
- WAT-18 无 `#[ignore]` 且 green

### R-watch-008: overrides 无静默回退

动态 set 的值不被缓存 TTL 静默回退为 merged 旧值。
**验收标准：**
- set 后任意时刻读取仍为新值

### R-watch-009: 渐进发布可观测且死配置接线

`peek_candidate()` 暴露 trial 期候选；`rollback_on_validation_failure` 真实控制校验失败回滚；MigrationOnReload 在提交成功后执行迁移。
**验收标准：**
- begin_reload 期间 peek_candidate 返回 Some
- 校验失败回滚行为受该开关控制（true=回滚保留旧值）
- reload 提交后 registry migrate 被调用

## Constraints

- 不改变 arc-swap 读侧快照一致性与取消安全既有契约（既有测试不得回归）。
- panic 隔离用 `AssertUnwindSafe` 并注释理由。

## Out of Scope

- 多文件聚合发布窗口策略（保持使用方拼装）、跨进程配置推送。
