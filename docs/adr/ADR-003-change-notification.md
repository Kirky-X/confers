# ADR-003: 变更通知机制

## 状态
Accepted

## 背景
需要一种机制让应用能够感知配置变更。

## 决策
使用 `tokio::sync::watch` 作为配置变更通知机制。

## 后果
- 正面：简单易用，与 tokio 集成良好
- 正面：支持多订阅者
- 负面：不支持广播（需要 ConfigBus）

## 相关
- ADR-035: ConfigBus 多实例广播
