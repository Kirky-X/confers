# ADR-011: 测试策略

## 状态
Accepted

## 背景
需要系统化的测试策略。

## 决策
- 单元测试：核心模块
- 属性测试：proptest
- 集成测试：Docker 环境
- 覆盖率：全局 80%，核心模块 95%+

## 后果
- 正面：高测试覆盖率保证质量
- 负面：测试维护成本

---

# ADR-012: CI/CD

## 状态
Accepted

## 背景
需要自动化构建、测试、发布流程。

## 决策
- GitHub Actions
- tag-push 发布
- ADR 一致性检查

## 后果
- 正面：自动化保证质量
- 负面：CI 配置复杂度

---

# ADR-013: 重载安全

## 状态
Accepted

## 背景
配置热重载需要保证安全。

## 决策
- 预检：文件大小、格式、必填键
- 防抖：notify-debouncer-full
- 验证：garde 验证
- 原子替换

## 后果
- 正面：热重载安全可靠
- 负面：重载延迟增加

---

# ADR-014: API 边界隔离

## 状态
Accepted

## 背景
需要清晰的 API 边界。

## 决策
- #![deny(missing_docs)]
- 最小接口原则

## 后果
- 正面：API 清晰易用
- 负面：文档工作量

---

# ADR-015: 密钥轮转

## 状态
Accepted

## 背景
需要支持密钥轮转。

## 决策
- HKDF 派生
- KeyRegistry 管理
- 两阶段：prepare → commit
- 检查点恢复

## 后果
- 正面：安全合规
- 负面：实现复杂度

---

# ADR-016: 配置安全

## 状态
Accepted

## 背景
需要配置安全机制。

## 决策
- KeyProvider 优先级
- OverridePolicy 覆盖策略

## 后果
- 正面：配置安全可控
- 负面：学习成本

---

# ADR-017: 优先级链

## 状态
Accepted

## 背景
需要明确的配置优先级。

## 决策
CLI > ENV > .env > File > Profile > Remote > $HOME > /etc > Default

## 后果
- 正面：优先级明确
- 负面：需要文档说明

---

# ADR-018: 敏感字段保护

## 状态
Accepted

## 背景
敏感字段需要保护。

## 决策
- sensitive 自动 DenyList(Remote)
- encrypt 隐含 sensitive

## 后果
- 正面：防止敏感信息泄露
- 负面：可能限制灵活性

---

# ADR-019: 密钥强度验证

## 状态
Accepted

## 背景
需要验证密钥强度。

## 决策
- 长度检查
- 熵值检测
- 弱密钥识别

## 后果
- 正面：防止弱密钥
- 负面：可能误判

---

# ADR-020: 安全最佳实践

## 状态
Accepted

## 背景
需要安全最佳实践指导。

## 决策
见 dev-v2.md 第 19 节。

## 后果
- 正面：安全有据可依
