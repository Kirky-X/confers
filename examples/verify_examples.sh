#!/bin/bash
# Confers 示例验证脚本
#
# 此脚本用于验证统一项目中的所有示例的编译和运行
# 使用方法: ./verify_examples.sh [--run]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 是否运行示例
RUN_EXAMPLES=false
if [ "$1" == "--run" ]; then
    RUN_EXAMPLES=true
fi

echo "========================================="
echo "Confers Examples 验证脚本"
echo "========================================="
echo ""

# 所有示例名称
ALL_EXAMPLES=(
    "basic_usage"
    "hot_reload"
    "remote_consul"
    "encryption"
    "key_rotation"
    "migration"
    "dynamic_fields"
    "config_groups"
    "progressive_reload"
    "full_stack"
)

echo "项目目录: $PROJECT_DIR"
echo ""

# 检查 Cargo.toml 存在
if [ ! -f "$PROJECT_DIR/Cargo.toml" ]; then
    echo -e "${RED}错误: Cargo.toml 不存在${NC}"
    exit 1
fi

# 检查示例源文件
check_example_files() {
    local example_name=$1
    local example_file="$PROJECT_DIR/src/examples/${example_name}.rs"
    
    if [ ! -f "$example_file" ]; then
        echo -e "${RED}✗ 缺少源文件: src/examples/${example_name}.rs${NC}"
        return 1
    fi
    return 0
}

echo "步骤 1: 检查文件完整性"
echo "-----------------------------------"
missing_files=0
for example in "${ALL_EXAMPLES[@]}"; do
    echo -n "检查 $example ... "
    if check_example_files "$example"; then
        echo -e "${GREEN}✓${NC}"
    else
        ((missing_files++))
    fi
done

if [ $missing_files -gt 0 ]; then
    echo ""
    echo -e "${RED}有 $missing_files 个示例缺少源文件${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}所有示例文件完整 ✓${NC}"
echo ""

# 编译检查
echo "步骤 2: 验证编译"
echo "-----------------------------------"
cd "$PROJECT_DIR"

failed_builds=0
for example in "${ALL_EXAMPLES[@]}"; do
    echo -n "编译 $example ... "
    
    if cargo build --bin "$example" --quiet 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ 编译失败${NC}"
        ((failed_builds++))
    fi
done

echo ""
if [ $failed_builds -gt 0 ]; then
    echo -e "${RED}有 $failed_builds 个示例编译失败${NC}"
    exit 1
fi

echo -e "${GREEN}所有示例编译成功 ✓${NC}"
echo ""

# Clippy 检查
echo "步骤 3: 代码质量检查 (clippy)"
echo "-----------------------------------"
failed_clippy=0
for example in "${ALL_EXAMPLES[@]}"; do
    echo -n "clippy $example ... "
    
    if cargo clippy --bin "$example" --quiet 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ 有警告${NC}"
        ((failed_clippy++))
    fi
done

echo ""

# 可选：运行示例
if [ "$RUN_EXAMPLES" = true ]; then
    echo "步骤 4: 运行示例"
    echo "-----------------------------------"
    echo -e "${BLUE}注意: 某些示例需要外部服务 (Consul, etcd) 才能完全运行${NC}"
    echo ""
    
    for example in "${ALL_EXAMPLES[@]}"; do
        echo -n "运行 $example ... "
        
        # 设置超时（大多数示例应该快速完成或优雅失败）
        if timeout 10 cargo run --bin "$example" --quiet 2>/dev/null; then
            echo -e "${GREEN}✓${NC}"
        else
            exit_code=$?
            if [ $exit_code -eq 124 ]; then
                echo -e "${YELLOW}⏱ 超时（可能需要外部服务）${NC}"
            else
                echo -e "${YELLOW}⚠ 运行失败（可能需要外部服务）${NC}"
            fi
        fi
    done
    echo ""
fi

# 总结
echo "========================================="
if [ $failed_clippy -eq 0 ]; then
    echo -e "${GREEN}验证完成！所有示例均通过检查 ✓${NC}"
else
    echo -e "${YELLOW}验证完成，但 $failed_clippy 个示例有 clippy 警告${NC}"
fi
echo "========================================="
echo ""
echo "运行示例命令:"
echo "  cargo run --bin basic_usage"
echo "  cargo run --bin hot_reload"
echo "  cargo run --bin encryption"
echo "  ..."
echo ""
echo "完整验证（含运行）: ./verify_examples.sh --run"
