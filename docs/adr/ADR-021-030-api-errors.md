# ADR-021: 错误消息分级

## 状态
Accepted

## 背景
不同场景需要不同的错误消息详细程度。

## 决策
- user_message(): 用户显示级别
- sanitized_chain(): 错误链脱敏
- internal_message(): 开发调试（仅 debug）
- audit_message(): 审计日志

## 后果
- 正面：灵活性与安全性平衡

---

# ADR-022: 远程源安全

## 状态
Accepted

## 背景
远程源需要安全传输。

## 决策
- 强制 HTTPS
- TLS 配置
- Ed25519 签名验证

## 后果
- 正面：传输安全
- 负面：配置复杂度

---

# ADR-023: 密钥轮转一致性

## 状态
Accepted

## 背景
密钥轮转需要保证一致性。

## 决策
- 启动检查
- repair 机制
- 审计日志

## 后果
- 正面：一致性保证

---

# ADR-024: 增量合并优化

## 状态
Accepted

## 背景
需要优化合并性能。

## 决策
- 路径索引
- 反向索引

## 后果
- 正面：合并性能提升

---

# ADR-025: 自适应防抖

## 状态
Accepted

## 背景
文件监视需要智能防抖。

## 决策
- notify-debouncer-full 平台层防抖
- AdaptiveDebouncer 应用层自适应

## 后果
- 正面：减少不必要的重载

---

# ADR-026: 自定义值树

## 状态
Accepted

## 背景
需要自定义值树替代 serde_json::Value。

## 决策
AnnotatedValue（含 SourceLocation）替代裸 serde_json::Value。

## 后果
- 正面：精确值溯源
- 负面：实现复杂度

---

# ADR-027: 配置版本化

## 状态
Accepted

## 背景
需要配置版本迁移。

## 决策
- Versioned trait
- MigrationRegistry
- 预计算迁移路径

## 后果
- 正面：平滑升级

---

# ADR-028: API 边界隔离

## 状态
Accepted

## 背景
需要 API 边界隔离。

## 决策
SecretString newtype 封装 secrecy。

## 后果
- 正面：内存安全

---

# ADR-029: KeyProvider 拆分

## 状态
Accepted

## 背景
需要区分同步/异步密钥提供。

## 决策
- KeyProvider (sync)
- AsyncKeyProvider (async)

## 后果
- 正面：职责分离

---

# ADR-030: OverridePolicy

## 状态
Accepted

## 背景
需要覆盖策略控制。

## 决策
- Unrestricted
- MinPriority
- Locked
- AllowList/DenyList

## 后果
- 正面：细粒度控制
