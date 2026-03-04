# ADR-004: 合并策略

## 状态
Accepted

## 背景
需要统一的合并策略来处理多源配置冲突。

## 决策
实现七种合并策略：
- Replace: 高优先级替换
- Join: 保留低优先级
- Append: 追加到数组末尾
- Prepend: 前置到数组开头
- JoinAppend: 连接两个数组
- DeepMerge: 按索引匹配合并
- Custom: 自定义函数

## 后果
- 正面：覆盖大多数合并场景
- 负面：学习成本增加
