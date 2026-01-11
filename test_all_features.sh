#!/bin/bash
# 测试所有特性组合的编译

set -e

echo "🧪 测试所有特性组合..."
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 测试计数器
TOTAL=0
PASSED=0
FAILED=0

# 测试函数
test_feature() {
    local name=$1
    local features=$2
    local target=$3  # lib or bin

    TOTAL=$((TOTAL + 1))
    echo -n "测试 $name (features=$features, target=$target)... "

    if [ "$target" = "lib" ]; then
        build_cmd="cargo build --no-default-features --features $features --lib"
    else
        build_cmd="cargo build --no-default-features --features $features"
    fi

    if eval $build_cmd 2>&1 | grep -q "Finished"; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ FAILED${NC}"
        FAILED=$((FAILED + 1))
    fi
}

# 测试库编译
echo "📦 测试库编译..."
echo ""

test_feature "minimal" "minimal" "lib"
test_feature "recommended" "recommended" "lib"
test_feature "dev" "dev" "lib"
test_feature "production" "production" "lib"
test_feature "full" "full" "lib"

echo ""
echo "🔧 测试二进制编译..."
echo ""

test_feature "cli" "cli" "bin"
test_feature "dev (binary)" "dev" "bin"
test_feature "full (binary)" "full" "bin"

echo ""
echo "📊 测试结果汇总:"
echo "=================="
echo "总计: $TOTAL"
echo -e "通过: ${GREEN}$PASSED${NC}"
echo -e "失败: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}❌ 有 $FAILED 个测试失败${NC}"
    exit 1
fi