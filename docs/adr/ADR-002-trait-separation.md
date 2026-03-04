# ADR-002: Trait sync/async 分离

## 状态
Accepted

## 背景
需要明确同步和异步配置访问的职责边界。

## 决策
- `ConfigProvider`: 同步接口，用于本地配置访问
- `AsyncConfigProvider`: 异步接口，用于远程配置动态访问
- `KeyProvider`: 同步密钥提供（文件、环境变量）
- `AsyncKeyProvider`: 异步密钥提供（Vault、KMS）

## 后果
- 正面：职责清晰，避免不必要的异步开销
- 正面：用户只需实现必要接口
- 负面：需要根据场景选择正确的 trait
