<div align="center">

<img src="../resource/confers.png" alt="Confers Logo" width="150" style="margin-bottom: 16px;">

# 🏗️ 架构决策记录 (ADR)

### Confers 项目重要架构决策的记录

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [📚 API 参考](API_REFERENCE.md)

---

</div>

## 📋 目录

<details open style="background:#F8FAFC; border-radius:8px; padding:16px; border:1px solid #E2E8F0;">
<summary style="cursor:pointer; font-weight:600; color:#1E293B;">📑 目录（点击展开）</summary>

- [1. 加密策略](#1-加密策略)
- [2. Nonce 缓存大小](#2-nonce-缓存大小)
- [3. 提供者优先级系统](#3-提供者优先级系统)
- [4. 内存限制](#4-内存限制)
- [5. 配置验证方法](#5-配置验证方法)
- [6. 默认值语法](#6-默认值语法)
- [附录：决策模板](#附录决策模板)

</details>

---

## 1. 加密策略

<div style="background:#DCFCE7; border-radius:8px; padding:16px; border:1px solid #86EFAC; margin: 16px 0;">

**状态**: 已实现 | **日期**: 2025-01-11 | **上下文**: 敏感配置值的安全存储

</div>

### 问题陈述

配置值可能包含敏感信息（API 密钥、密码、令牌），需要：

- 🔐 静态加密 - 文件中的敏感数据加密
- 🧹 内存安全 - 防止内存检查
- 🛡️ 重放攻击防护 - Nonce 重用检测

### 决策

**选择**: 带有 Nonce 重用检测的 AES-256-GCM

### 替代方案

| 替代方案 | 优点 | 缺点 | 拒绝原因 |
|:---------|:-----|:-----|:---------|
| AES-256-CBC | 广泛支持 | 无认证加密 | 无完整性验证 |
| ChaCha20-Poly1305 | 现代 AEAD | 库支持较少 | 实现复杂度高 |
| RSA/ECDSA | 密钥交换 | 性能慢 | 不适合配置值 |
| XChaCha20-Poly1305 | 扩展 Nonce | 库支持较少 | 与 ChaCha20 类似 |

### 理由

1. **安全性**: AES-256-GCM 提供认证加密（机密性 + 完整性 + 真实性）
2. **性能**: 现代 CPU 支持硬件加速（AES-NI）
3. **库支持**: Rust 生态系统中广泛支持（`aes-gcm` crate）
4. **标准化**: NIST 批准的算法（FIPS 197）
5. **Nonve 管理**: 96 位 Nonce 提供 ~2^96 个唯一值，足以应对配置生命周期

### 权衡

- **Nonce 缓存大小**: 10,000 个条目使用约 1.2MB 内存（每个条目 120 字节）
  - 在 1-30 秒重新加载间隔下可支持约 2-4 小时操作
  - LRU 驱逐确保无界增长
  - 加密检查仍能在驱逐后检测到重用

### 参考

- NIST SP 800-38D: 块密码工作模式建议
- RFC 5116: AES-GCM 密码及其在 IPsec 中的使用

---

## 2. Nonce 缓存大小

<div style="background:#DBEAFE; border-radius:8px; padding:16px; border:1px solid #93C5FD; margin: 16px 0;">

**状态**: 已实现 | **日期**: 2025-01-11 | **上下文**: 安全性与内存使用的平衡

</div>

### 问题陈述

Nonce 重用检测需要跟踪所有使用的 Nonce。无界增长将：

- 消耗无限内存
- 通过配置注入攻击导致潜在的 DoS

### 决策

**选择**: 限制为 10,000 个条目的 LRU 缓存

### 替代方案

| 替代方案 | 优点 | 缺点 | 拒绝原因 |
|:---------|:-----|:-----|:---------|
| 无界 HashSet | 无限检测 | 内存 DoS 漏洞 | 安全风险太高 |
| 1,000 个条目 | 更低内存 | 可能不覆盖典型使用 | 限制性太强 |
| 100,000 个条目 | 更好的覆盖范围 | 更高内存 (~12MB) | 不值得成本 |
| 基于时间的过期 | 自动清理 | 复杂实现 | 难以调优超时 |

### 理由

1. **安全性**: 10,000 个条目为典型场景提供充足的检测
2. **内存**: 1.2MB 对于配置管理库来说是可接受的
3. **LRU 驱逐**: 将最近的 Nonce 保留在内存中（热路径优化）
4. **双重保护**: LRU 驱逐 + 加密检查提供深度防御
5. **条目大小**: 120 字节（Nonce + 时间戳）是合理的

### 权衡

- **高频重新加载**: 每秒重新加载配置，持续数小时可能耗尽缓存
- **解决方案**: 记录推荐的重新加载间隔（5-60 秒）

### 未来考虑

如果高频重新加载成为需求，考虑：
- 基于时间的驱逐（例如，超过 1 小时的 Nonce）
- 每个提供者的 Nonce 池
- 基于使用模式的自适应缓存大小调整

---

## 3. 提供者优先级系统

<div style="background:#FEF3C7; border-radius:8px; padding:16px; border:1px solid #FCD34D; margin: 16px 0;">

**状态**: 已实现 | **日期**: 2025-01-11 | **上下文**: 配置可来自多个来源

</div>

### 问题陈述

当多个提供者返回相同的配置键时，应使用哪个值？

### 决策

**选择**: 数字优先级系统（数字越高 = 优先级越高）

### 替代方案

| 替代方案 | 优点 | 缺点 | 拒绝原因 |
|:---------|:-----|:-----|:---------|
| 先到先得 | 简单 | 顺序不灵活 | 用户控制有限 |
| 后到先得 | 最近的值为王 | 文件顺序无关 | 不友好 |
| 加权平均 | 公平分布 | 复杂配置 | 难以预测行为 |

### 默认优先级

<div style="background:#F8FAFC; border-radius:8px; padding:16px; border:1px solid #E2E8F0;">

| 提供者 | 优先级 | 描述 |
|:-------|:------:|:-----|
| FileConfigProvider | 10 | 文件提供者（最高优先级） |
| CliConfigProvider | 20 | CLI 提供者 |
| EnvironmentProvider | 30 | 环境变量提供者 |
| HttpConfigProvider | 30 | HTTP 提供者（可配置） |
| ConsulConfigProvider | 30 | Consul 提供者 |
| EtcdConfigProvider | 30 | Etcd 提供者 |

</div>

---

## 4. 内存限制

<div style="background:#EDE9FE; border-radius:8px; padding:16px; border:1px solid #A78BFA; margin: 16px 0;">

**状态**: 已实现 | **日期**: 2025-01-11 | **上下文**: 防止配置文件导致内存耗尽

</div>

### 问题陈述

大型配置文件或恶意输入可能导致：

- 解析时消耗过多内存
- 导致应用程序崩溃
- 通过配置注入启用 DoS 攻击

### 决策

**选择**: 可配置的内存限制（默认: 512MB）并强制执行

### 替代方案

| 替代方案 | 优点 | 缺点 | 拒绝原因 |
|:---------|:-----|:-----|:---------|
| 无限制 | 最佳性能 | 安全漏洞 | DoS 风险太高 |
| 硬限制 | 简单 | 不灵活 | 不适应用例 |
| 基于百分比 | 相对 | 难以正确设置 | 复杂的用户配置 |
| 每个提供者限制 | 粒度 | 复杂实现 | 过度设计 |

### 理由

1. **安全性**: 512MB 限制防止大多数 DoS 攻击，同时保持合理
2. **灵活性**: 用户可以通过 `ConfigLoader::with_memory_limit()` 增加限制
3. **可预测性**: 固定大小允许更好的容量规划
4. **实现**: 使用 `sysinfo` 易于跟踪和执行

### 权衡

- **大型配置文件**: 用户必须增加限制
- **动态分配**: 可能在增长阶段拒绝有效（但大型）的配置
- **平台差异**: 内存跟踪因操作系统而异

---

## 5. 配置验证方法

<div style="background:#FEE2E2; border-radius:8px; padding:16px; border:1px solid #FCA5A5; margin: 16px 0;">

**状态**: 已实现 | **日期**: 2025-01-11 | **上下文**: 确保配置值满足应用程序要求

</div>

### 问题陈述

配置值可能有约束：

- 类型验证（例如，端口是 u16）
- 业务规则（例如，端口 > 1024）
- 跨字段验证（例如，db_url 取决于 db_type）
- 自定义验证逻辑

### 决策

**选择**: 使用 `validator` crate 的声明式验证，支持自定义验证器

### 替代方案

| 替代方案 | 优点 | 缺点 | 拒绝原因 |
|:---------|:-----|:-----|:---------|
| 手动 if 检查 | 简单 | 样板代码，运行时错误 | 不可维护 |
| 过程宏 | 更少的样板代码 | 难以调试 | 难以自定义 |
| 类型状态机 | 强保证 | 复杂 | 对于配置过度设计 |
| 自定义派生宏 | 完美集成 | 复杂实现 | 重新实现 validator |

### 实现示例

```rust
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config, Validate)]
#[config(env_prefix = "APP")]
#[config(validate)]
struct AppConfig {
    #[config(default = 8080)]
    #[validate(range(min = 1024, max = 65535))]
    port: u16,

    #[config(default = "\"localhost\".to_string()")]
    #[validate(url(port = 8080))]
    server_url: String,

    #[config(custom_validate = "validate_secret")]
    #[config(sensitive = true)]
    api_key: String,
}
```

---

## 6. 默认值语法

<div style="background:#DCFCE7; border-radius:8px; padding:16px; border:1px solid #86EFAC; margin: 16px 0;">

**状态**: 已实现 | **日期**: 2025-01-11 | **上下文**: 简化配置字段的默认值指定

</div>

### 旧方法（需要复杂语法）

```rust
#[derive(Config)]
struct Config {
    // 冗长且容易出错
    #[config(default = "\"hello\".to_string()")]
    message: String,
}
```

### 新方法（简单语法）

```rust
#[derive(Config)]
struct Config {
    // 干净简单
    #[config(default = "hello")]
    message: String,

    // 对所有类型都正确工作
    #[config(default = 42)]
    number: u32,
}
```

### 理由

1. **用户体验**: `default = "hello"` 比 `default = "\"hello\".to_string()"` 更自然
2. **类型安全**: 通过 `.to_string()` 自动将 `&str` 文字转换为 `String`
3. **向后兼容**: 旧的 `.to_string()` 语法仍然有效
4. **实现**: 宏代码生成中的简单模式匹配

### 好处

| 好处 | 描述 |
|:-----|:-----|
| 减少样板代码 | 字符串默认值用户代码减少 60% |
| 更少错误 | 编译器捕获类型不匹配，而不是运行时 panic |
| 更好的可读性 | 配置定义更清晰、更简洁 |
| IDE 支持 | 更好的自动完成和语法高亮 |

---

## 附录：决策模板

<div style="background:#F8FAFC; border-radius:8px; padding:16px; border:1px solid #E2E8F0; margin: 16px 0;">

```markdown
## [N]. [标题]

**状态**: [提议 | 已接受 | 已废弃 | 已替代]
**日期**: YYYY-MM-DD
**上下文**: 问题或情况的简要描述

### 问题陈述

我们要解决什么问题？

### 决策

所选方法的简要总结。

### 替代方案

| 替代方案 | 优点 | 缺点 | 拒绝原因 |
|:---------|:-----|:-----|:---------|

### 理由

为什么做出这个决定？有什么后果？

### 权衡

缺点或妥协是什么？

### 实现说明

任何相关的代码片段或实现细节。

### 未来考虑

以后应该重新考虑什么？
```

</div>

---

## 如何添加新决策

1. 从附录复制模板
2. 填写所有部分
3. 赋予一个顺序编号
4. 提交消息: `docs(adrs): add decision for [topic]`
5. 在目录中添加简要摘要
6. 更新此文件的最后修改日期

---

<div align="center" style="margin: 32px 0; padding: 24px; background: linear-gradient(135deg, #FEF3C7 0%, #EDE9FE 100%); border-radius: 12px;">

### 📖 了解更多

**[📖 用户指南](USER_GUIDE.md)** • **[📚 API 参考](API_REFERENCE.md)** • **[🏠 首页](../README.md)**

由 Confers 团队用 ❤️ 制作

**[⬆ 返回顶部](#架构决策记录-adr)**

</div>