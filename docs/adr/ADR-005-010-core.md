# ADR-005: 远程源

## 状态
Accepted

## 背景
应用需要从远程配置中心加载配置。

## 决策
支持 Consul 和 etcd 作为远程配置源，可同时启用。

## 后果
- 正面：支持主流远程配置中心
- 负面：增加依赖体积

---

# ADR-006: 密钥边界

## 状态
Accepted

## 背景
需要安全的密钥管理边界。

## 决策
- SecretString newtype 封装
- XChaCha20-Poly1305 加密
- HKDF-SHA256 派生字段级密钥

## 后果
- 正面：内存安全，零拷贝解密
- 负面：需要用户配合使用 SecretString

---

# ADR-007: 宏 API

## 状态
Accepted

## 背景
需要简洁的配置派生宏。

## 决策
使用 `#[derive(Config)]` + `darling` 属性解析。

## 后果
- 正面：零样板代码
- 负面：宏调试困难

---

# ADR-008: 错误设计

## 状态
Accepted

## 背景
需要统一的错误处理。

## 决策
- ConfigError 枚举
- 分级消息：user_message, sanitized_chain, internal_message, audit_message
- 安全敏感错误不携带详情

## 后果
- 正面：安全性与可调试性平衡
- 负面：错误处理代码量增加

---

# ADR-009: 特性图

## 状态
Accepted

## 背景
需要可裁剪的功能集。

## 决策
见 dev-v2.md 第 5 节特性标志参考。

## 后果
- 正面：按需裁剪二进制体积
- 负面：特性组合测试复杂

---

# ADR-010: 可观测性

## 状态
Accepted

## 背景
需要可观测性支持。

## 决策
- tracing 用于结构化日志
- metrics 用于指标

## 后果
- 正面：与现代可观测性栈兼容
