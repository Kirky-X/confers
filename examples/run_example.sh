#!/bin/bash
# 快速运行单个示例
#
# 用法: ./run_example.sh <example_name> [args...]

EXAMPLE_NAME=$1

if [ -z "$EXAMPLE_NAME" ]; then
    echo "用法: $0 <example_name> [args...]"
    echo ""
    echo "可用的示例:"
    echo "  - basic_usage"
    echo "  - hot_reload"
    echo "  - remote_consul"
    echo "  - encryption"
    echo "  - key_rotation"
    echo "  - migration"
    echo "  - dynamic_fields"
    echo "  - config_groups"
    echo "  - progressive_reload"
    echo "  - full_stack"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 检查示例源文件是否存在
if [ ! -f "$SCRIPT_DIR/src/examples/${EXAMPLE_NAME}.rs" ]; then
    echo "错误: 示例 '$EXAMPLE_NAME' 不存在"
    exit 1
fi

echo "========================================="
echo "运行示例: $EXAMPLE_NAME"
echo "========================================="
echo ""

# 检查是否有单独的 README
if [ -f "$SCRIPT_DIR/src/examples/${EXAMPLE_NAME}.md" ]; then
    echo "📖 示例说明:"
    echo "-----------------------------------"
    head -20 "$SCRIPT_DIR/src/examples/${EXAMPLE_NAME}.md"
    echo "..."
    echo ""
fi

cd "$SCRIPT_DIR"

# 运行示例
echo "🚀 运行示例..."
echo "-----------------------------------"
cargo run --bin "$EXAMPLE_NAME" -- "${@:2}"

echo ""
echo "========================================="
echo "示例运行完成"
echo "========================================="
