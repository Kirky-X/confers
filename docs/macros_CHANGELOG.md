# 更新日志

本文件记录本项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，
版本号遵循 [语义化版本 2.0.0](https://semver.org/spec/v2.0.0.html)。

## [Unreleased]

### 新增

- 全面的安全模块，包含路径穿越防护
- 基于 `secrecy` crate 的敏感数据保护
- 类型安全的 Schema 生成
- 完善的测试套件（覆盖率目标 80% 以上）
- 类型解析性能优化（速度提升 50% 以上）
- 统一的错误处理与详尽的错误信息
- 完整的 API 文档与示例
- 对所有用户提供的字符串进行输入校验
- 用于优化类型处理的类型类别枚举

### 变更

- **破坏性**：加强密钥文件（secret file）的路径校验（更加严格）
- **破坏性**：敏感字段默认使用 `SecretString`
- 重构代码生成，提升可维护性
- 统一所有派生宏的 API 命名规范
- 编译性能提升 40% 以上
- 使用模式匹配替代字符串比较，优化类型检测
- 增强输入校验：引入长度限制与字符白名单

### 修复

- 安全漏洞：密钥文件加载中的路径穿越问题
- 类型字符串缓存中的内存泄漏
- 嵌套 `Option` 类型的 Schema 生成错误
- 缺失的加密算法校验
- 重复类型字符串转换导致的性能问题

### 安全

- 为 `_FILE` 环境变量新增路径穿越防护
- 新增敏感数据内存清零（zeroizing）
- 新增对所有用户提供的字符串的输入校验
- 限制密钥文件允许的目录范围
- 新增 URL 编码穿越检测
- 新增路径最大长度校验

## [0.3.0] - 2024-01-15

### 新增

- 首次发布
- `Config` 派生宏，用于配置加载
- `ConfigSchema` 派生宏，用于 JSON Schema 生成
- `ConfigMigration` 派生宏，用于版本迁移
- `ConfigModules` 派生宏，用于模块分组
- `ConfigClap` 派生宏，用于 CLI 参数解析
- 支持前缀的环境变量加载
- 默认值支持
- 敏感字段处理
- 基于文件的密钥加载（`_FILE` 后缀）
- 通过 `flatten` 支持嵌套配置
- 与 `garde` 的校验集成

[Unreleased]: https://github.com/Kirky-X/confers/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Kirky-X/confers/releases/tag/v0.3.0
