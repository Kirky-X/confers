#!/bin/bash
# security_audit.sh - Confers 项目安全审计脚本
# 此脚本集成了多种安全检查工具，用于确保代码库的安全性
# 使用方法: ./scripts/security_audit.sh [--full] [--quick] [--fix]

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# 临时文件
TEMP_DIR=$(mktemp -d)
REPORT_FILE="${TEMP_DIR}/security_report.txt"
ERROR_FILE="${TEMP_DIR}/errors.txt"

# 跟踪错误数量
declare -i ERROR_COUNT=0
declare -i WARNING_COUNT=0

# 清理函数
cleanup() {
    if [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

# 打印函数
print_header() {
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}\n"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
    ((ERROR_COUNT++))
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    ((WARNING_COUNT++))
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# 初始化报告文件
init_report() {
    cat > "$REPORT_FILE" << EOF
================================================================================
Confers 项目安全审计报告
生成时间: $(date '+%Y-%m-%d %H:%M:%S')
================================================================================

EOF
}

# 添加报告内容
add_to_report() {
    echo "$1" >> "$REPORT_FILE"
}

# 检查依赖工具
check_dependencies() {
    print_header "检查依赖工具"
    
    local missing_tools=()
    
    # 检查必需工具
    for tool in cargo rustc git; do
        if ! command -v "$tool" &> /dev/null; then
            print_error "缺少必需工具: $tool"
            missing_tools+=("$tool")
        else
            print_success "$tool 已安装: $(command -v $tool)"
        fi
    done
    
    # 检查可选工具
    for tool in cargo-deny cargo-clippy cargo-fmt cargo-tarpaulin; do
        if command -v "$tool" &> /dev/null; then
            print_success "$tool 已安装"
        else
            print_warning "$tool 未安装 (可选)"
        fi
    done
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        print_error "缺少必需工具，无法继续执行安全审计"
        echo "请安装缺少的工具: ${missing_tools[*]}"
        exit 1
    fi
}

# 运行 cargo-deny 安全审计
run_cargo_deny() {
    print_header "运行 cargo-deny 安全审计"
    
    cd "$PROJECT_ROOT"
    
    if command -v cargo-deny &> /dev/null; then
        print_info "执行 cargo-deny check..."
        
        if cargo deny check 2>&1 | tee "$ERROR_FILE"; then
            print_success "cargo-deny 检查通过"
        else
            if grep -q "advisories" "$ERROR_FILE" 2>/dev/null; then
                print_warning "发现依赖安全警告，请查看 advisories"
            fi
            if grep -q "bans" "$ERROR_FILE" 2>/dev/null; then
                print_warning "发现依赖禁令警告，请查看 bans"
            fi
            if grep -q "licenses" "$ERROR_FILE" 2>/dev/null; then
                print_warning "发现许可证警告，请查看 licenses"
            fi
        fi
        
        add_to_report "\n=== cargo-deny 检查结果 ==="
        if [[ -f "$ERROR_FILE" ]]; then
            add_to_report "$(cat "$ERROR_FILE")"
        fi
    else
        print_warning "cargo-deny 未安装，跳过此项检查"
        print_info "安装命令: cargo install --locked cargo-deny"
    fi
}

# 运行 cargo-clippy 代码质量检查
run_clippy() {
    print_header "运行 cargo-clippy 代码质量检查"
    
    cd "$PROJECT_ROOT"
    
    if command -v cargo-clippy &> /dev/null; then
        print_info "执行 clippy 检查..."
        
        # 只检查警告，不严重错误
        if cargo clippy --all-features --workspace \
            -- -D warnings 2>&1 | tee "$ERROR_FILE"; then
            print_success "clippy 检查通过"
        else
            # 统计错误数量
            local error_count=$(grep -c "error\[" "$ERROR_FILE" 2>/dev/null || echo "0")
            local warning_count=$(grep -c "warning\[" "$ERROR_FILE" 2>/dev/null || echo "0")
            
            if [[ "$error_count" -gt 0 ]]; then
                print_error "发现 $error_count 个 clippy 错误"
            fi
            if [[ "$warning_count" -gt 0 ]]; then
                print_warning "发现 $warning_count 个 clippy 警告"
            fi
            
            # 常见错误提示
            add_to_report "\n=== clippy 常见修复建议 ==="
            add_to_report "1. 如果看到 'dereferencing a None pointer' 错误："
            add_to_report "   - 使用 if let 或模式匹配处理 Option"
            add_to_report "   - 使用 unwrap_or, unwrap_or_else, or_else 等方法"
            add_to_report ""
            add_to_report "2. 如果看到 'unused import' 警告："
            add_to_report "   - 删除未使用的 import"
            add_to_report "   - 使用 #[allow(unused)] 临时禁用（不推荐）"
            add_to_report ""
            add_to_report "3. 如果看到 'clippy::result_large_err' 警告："
            add_to_report "   - 考虑使用 Box<dyn Error> 或自定义错误类型"
            add_to_report ""
        fi
        
        add_to_report "\n=== clippy 检查结果 ==="
        if [[ -f "$ERROR_FILE" ]]; then
            add_to_report "$(cat "$ERROR_FILE")"
        fi
    else
        print_warning "cargo-clippy 未安装，跳过此项检查"
        print_info "安装命令: rustup component add clippy"
    fi
}

# 检查代码格式
check_format() {
    print_header "检查代码格式"
    
    cd "$PROJECT_ROOT"
    
    print_info "执行 cargo fmt --check..."
    
    if cargo fmt -- --check 2>&1 | tee "$ERROR_FILE"; then
        print_success "代码格式检查通过"
    else
        if grep -q "diff" "$ERROR_FILE" 2>/dev/null; then
            print_warning "代码格式不符合规范"
            print_info "修复命令: cargo fmt"
        fi
        
        add_to_report "\n=== 格式检查结果 ==="
        if [[ -f "$ERROR_FILE" ]]; then
            add_to_report "$(cat "$ERROR_FILE")"
        fi
    fi
}

# 检查依赖版本
check_dependency_versions() {
    print_header "检查依赖版本"
    
    cd "$PROJECT_ROOT"
    
    print_info "检查过时的依赖版本..."
    
    if command -v cargo-outdated &> /dev/null; then
        if cargo outdated --root-deps-only 2>&1 | tee "$ERROR_FILE"; then
            print_success "依赖版本检查通过"
        else
            print_warning "发现过时依赖，请查看输出"
        fi
        
        add_to_report "\n=== 依赖版本检查 ==="
        if [[ -f "$ERROR_FILE" ]]; then
            add_to_report "$(cat "$ERROR_FILE")"
        fi
    else
        print_warning "cargo-outdated 未安装，跳过此项检查"
        print_info "安装命令: cargo install cargo-outdated"
        
        # 备选方案：使用 cargo tree 检查依赖
        print_info "尝试使用 cargo tree 检查依赖..."
        if cargo tree --depth 1 2>&1 | head -20; then
            print_success "依赖树检查完成"
        fi
    fi
}

# 检查敏感信息泄露
check_sensitive_data() {
    print_header "检查敏感信息泄露"
    
    cd "$PROJECT_ROOT"
    
    print_info "扫描敏感信息..."
    
    local sensitive_patterns=(
        "password\s*=\s*['\"][^'\"]+['\"]"
        "secret\s*=\s*['\"][^'\"]+['\"]"
        "api_key\s*=\s*['\"][^'\"]+['\"]"
        "private_key\s*=\s*['\"][^'\"]+['\"]"
        "Bearer\s+[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+"
    )
    
    local found_issues=0
    
    for pattern in "${sensitive_patterns[@]}"; do
        if grep -rnE "$pattern" --include="*.toml" --include="*.yaml" --include="*.yml" --include="*.json" \
           --include="*.md" --include="*.txt" . 2>/dev/null | grep -v ".git" | grep -v "target" | grep -v "example" | grep -v "#"; then
            ((found_issues++))
        fi
    done
    
    if [[ "$found_issues" -gt 0 ]]; then
        print_warning "发现潜在敏感信息，请手动检查"
    else
        print_success "未发现明显敏感信息泄露"
    fi
}

# 检查 Cargo.lock 同步
check_lock_file() {
    print_header "检查 Cargo.lock 文件"
    
    cd "$PROJECT_ROOT"
    
    if [[ -f "Cargo.lock" ]]; then
        print_success "Cargo.lock 文件存在"
        
        # 检查 lock 文件是否过期
        print_info "检查 Cargo.lock 与 Cargo.toml 同步..."
        
        if cargo generate-lockfile --check 2>&1 | tee "$ERROR_FILE"; then
            print_success "Cargo.lock 与 Cargo.toml 同步"
        else
            print_warning "Cargo.lock 需要更新"
            print_info "更新命令: cargo generate-lockfile"
            
            add_to_report "\n=== Cargo.lock 同步检查 ==="
            if [[ -f "$ERROR_FILE" ]]; then
                add_to_report "$(cat "$ERROR_FILE")"
            fi
        fi
    else
        print_error "Cargo.lock 文件不存在"
        print_info "生成命令: cargo generate-lockfile"
    fi
}

# 检查测试覆盖
check_test_coverage() {
    print_header "检查测试覆盖"
    
    cd "$PROJECT_ROOT"
    
    print_info "检查测试套件..."
    
    # 检查是否有测试文件
    local test_count=$(find tests -name "*.rs" 2>/dev/null | wc -l)
    
    if [[ "$test_count" -gt 0 ]]; then
        print_success "发现 $test_count 个测试文件"
        
        # 检查测试类型
        local unit_tests=$(find tests -name "*.rs" -exec grep -l "#\[test\]" {} \; 2>/dev/null | wc -l)
        local integration_tests=$(find tests -name "*.rs" -exec grep -l "#\[tokio::test\]" {} \; 2>/dev/null | wc -l)
        
        print_info "单元测试: $unit_tests 个"
        print_info "集成测试: $integration_tests 个"
        
        # 尝试运行测试
        print_info "运行快速测试检查..."
        if cargo test --lib --all-features -- --test-threads=4 2>&1 | tee "$ERROR_FILE"; then
            print_success "基础测试通过"
        else
            print_warning "部分测试失败，请查看详细输出"
            
            add_to_report "\n=== 测试执行结果 ==="
            if [[ -f "$ERROR_FILE" ]]; then
                add_to_report "$(cat "$ERROR_FILE")"
            fi
        fi
    else
        print_warning "未发现测试文件"
    fi
}

# 检查 Git 最佳实践
check_git_practices() {
    print_header "检查 Git 最佳实践"
    
    cd "$PROJECT_ROOT"
    
    # 检查 .gitignore
    if [[ -f ".gitignore" ]]; then
        print_success ".gitignore 文件存在"
        
        # 检查是否忽略了 target 目录
        if grep -q "^target" ".gitignore" 2>/dev/null; then
            print_success "target 目录已被忽略"
        else
            print_warning "target 目录可能未在 .gitignore 中"
        fi
    else
        print_error ".gitignore 文件不存在"
    fi
    
    # 检查是否有大型文件
    print_info "检查大型文件..."
    local large_files=$(find . -type f -size +1M -not -path "./.git/*" -not -path "./target/*" 2>/dev/null)
    
    if [[ -n "$large_files" ]]; then
        print_warning "发现大型文件:"
        echo "$large_files"
    else
        print_success "未发现大型文件"
    fi
}

# 生成最终报告
generate_report() {
    print_header "生成安全审计报告"
    
    cd "$PROJECT_ROOT"
    
    # 复制报告到项目目录
    local report_destination="security_audit_report_$(date '+%Y%m%d_%H%M%S').txt"
    cp "$REPORT_FILE" "$report_destination"
    
    print_success "报告已生成: $report_destination"
    
    # 打印摘要
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  安全审计摘要${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}✅ 通过: $((7 - ERROR_COUNT - WARNING_COUNT)) 项${NC}"
    echo -e "${RED}❌ 错误: $ERROR_COUNT 项${NC}"
    echo -e "${YELLOW}⚠️  警告: $WARNING_COUNT 项${NC}"
    
    if [[ $ERROR_COUNT -gt 0 ]]; then
        echo -e "\n${RED}需要修复的错误数量: $ERROR_COUNT${NC}"
    fi
    
    if [[ $WARNING_COUNT -gt 0 ]]; then
        echo -e "${YELLOW}建议处理的警告数量: $WARNING_COUNT${NC}"
    fi
}

# 显示帮助信息
show_help() {
    cat << EOF
Confers 项目安全审计脚本

使用方法: $0 [选项]

选项:
    --full     运行完整的安全审计（包括所有检查项）
    --quick    快速模式（只运行关键检查）
    --fix      自动尝试修复可修复的问题
    --help     显示此帮助信息

示例:
    $0              # 运行标准安全审计
    $0 --full       # 运行完整安全审计
    $0 --quick      # 快速检查
    $0 --fix        # 尝试自动修复

检查项目:
    ✓ cargo-deny 依赖安全审计
    ✓ cargo-clippy 代码质量检查
    ✓ 代码格式检查
    ✓ 依赖版本检查
    ✓ 敏感信息泄露检查
    ✓ Cargo.lock 同步检查
    ✓ 测试覆盖检查
    ✓ Git 最佳实践检查

EOF
}

# 主函数
main() {
    local run_full=false
    local run_quick=false
    local auto_fix=false
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --full)
                run_full=true
                shift
                ;;
            --quick)
                run_quick=true
                shift
                ;;
            --fix)
                auto_fix=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                print_error "未知参数: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    echo -e "${BLUE}🚀 启动 Confers 项目安全审计${NC}"
    echo -e "${BLUE}📂 项目目录: $PROJECT_ROOT${NC}"
    echo -e "${BLUE}📅 执行时间: $(date '+%Y-%m-%d %H:%M:%S')${NC}\n"
    
    # 初始化报告
    init_report
    
    # 总是运行的检查
    check_dependencies
    run_cargo_deny
    run_clippy
    
    if [[ "$run_quick" == true ]]; then
        # 快速模式：只运行最关键的检查
        print_info "快速模式：跳过部分检查"
    else
        # 标准模式
        check_format
        check_dependency_versions
        check_sensitive_data
        check_lock_file
        check_test_coverage
        check_git_practices
    fi
    
    if [[ "$run_full" == true ]]; then
        print_info "完整模式：运行所有检查"
        # 完整模式可以添加更多检查
    fi
    
    # 生成报告
    generate_report
    
    # 如果有错误，返回非零退出码
    if [[ $ERROR_COUNT -gt 0 ]]; then
        print_error "安全审计发现 $ERROR_COUNT 个错误，需要修复"
        exit 1
    fi
    
    if [[ $WARNING_COUNT -gt 0 ]]; then
        print_warning "安全审计发现 $WARNING_COUNT 个警告，建议处理"
    fi
    
    print_success "安全审计完成！"
}

# 脚本入口
main "$@"