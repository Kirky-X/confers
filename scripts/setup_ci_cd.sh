#!/bin/bash
# setup_ci_cd.sh - Confers 项目 CI/CD 设置脚本
# 此脚本帮助开发者快速设置和配置所有 CI/CD 工具
# 使用方法: ./scripts/setup_ci_cd.sh [--install] [--update] [--check]

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}🚀 Confers CI/CD 设置脚本${NC}"
echo -e "${BLUE}📂 项目目录: $PROJECT_ROOT${NC}\n"

# 检查是否在项目根目录
check_project_root() {
    if [[ ! -f "Cargo.toml" ]]; then
        echo -e "${RED}❌ 错误: 必须在项目根目录运行此脚本${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ 项目根目录确认${NC}"
}

# 安装 Rust 工具
install_rust_tools() {
    echo -e "\n${BLUE}📦 安装 Rust 工具${NC}"
    
    # 安装 rustfmt
    echo "安装 rustfmt..."
    rustup component add rustfmt
    
    # 安装 clippy
    echo "安装 clippy..."
    rustup component add clippy
    
    # 安装 cargo-deny
    echo "安装 cargo-deny..."
    cargo install --locked cargo-deny
    
    # 安装 cargo-outdated
    echo "安装 cargo-outdated..."
    cargo install cargo-outdated
    
    # 安装 cargo-tarpaulin
    echo "安装 cargo-tarpaulin..."
    cargo install cargo-tarpaulin
    
    echo -e "${GREEN}✅ Rust 工具安装完成${NC}"
}

# 安装 Python 工具
install_python_tools() {
    echo -e "\n${BLUE}🐍 安装 Python 工具${NC}"
    
    # 检查 Python
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}❌ 错误: 需要安装 Python 3${NC}"
        exit 1
    fi
    
    # 安装 pre-commit
    echo "安装 pre-commit..."
    pip3 install --user pre-commit
    
    echo -e "${GREEN}✅ Python 工具安装完成${NC}"
}

# 设置 pre-commit hooks
setup_pre_commit() {
    echo -e "\n${BLUE}🪝 设置 Pre-commit Hooks${NC}"
    
    if ! command -v pre-commit &> /dev/null; then
        echo -e "${RED}❌ 错误: 未找到 pre-commit，请先运行 --install${NC}"
        exit 1
    fi
    
    # 初始化 hooks
    echo "初始化 pre-commit hooks..."
    pre-commit install
    
    # 安装 commit-msg hook
    echo "安装 commit-msg hook..."
    pre-commit install --hook-type commit-msg
    
    # 更新 hooks
    echo "更新 hooks 到最新版本..."
    pre-commit autoupdate
    
    # 运行初始检查
    echo "运行初始检查..."
    pre-commit run --all-files
    
    echo -e "${GREEN}✅ Pre-commit hooks 设置完成${NC}"
}

# 运行快速检查
run_quick_check() {
    echo -e "\n${BLUE}🔍 运行快速检查${NC}"
    
    # 格式检查
    echo "检查代码格式..."
    if cargo fmt --all -- --check; then
        echo -e "${GREEN}✅ 格式检查通过${NC}"
    else
        echo -e "${YELLOW}⚠️  格式检查失败，运行 cargo fmt 修复${NC}"
        cargo fmt --all
    fi
    
    # Clippy 检查
    echo "运行 clippy..."
    if cargo clippy --all-features --workspace -- -D warnings; then
        echo -e "${GREEN}✅ Clippy 检查通过${NC}"
    else
        echo -e "${YELLOW}⚠️  Clippy 发现问题${NC}"
    fi
    
    # 安全审计
    echo "运行安全审计..."
    if command -v cargo-deny &> /dev/null; then
        cargo deny check
    else
        echo -e "${YELLOW}⚠️  cargo-deny 未安装，跳过安全审计${NC}"
    fi
    
    echo -e "${GREEN}✅ 快速检查完成${NC}"
}

# 运行完整检查
run_full_check() {
    echo -e "\n${BLUE}🔍 运行完整检查${NC}"
    
    # 运行安全审计脚本
    echo "运行安全审计脚本..."
    if [[ -f "scripts/security_audit.sh" ]]; then
        ./scripts/security_audit.sh --full
    else
        echo -e "${YELLOW}⚠️  安全审计脚本未找到${NC}"
    fi
    
    # 运行测试
    echo "运行测试..."
    cargo test --all-features --workspace
    
    echo -e "${GREEN}✅ 完整检查完成${NC}"
}

# 验证配置
verify_config() {
    echo -e "\n${BLUE}✅ 验证配置文件${NC}"
    
    local configs=(
        ".github/workflows/ci.yml"
        ".github/workflows/ci-enhanced.yml"
        ".gitlab-ci.yml"
        ".pre-commit-config.yaml"
        "scripts/security_audit.sh"
        "deny.toml"
    )
    
    local all_valid=true
    
    for config in "${configs[@]}"; do
        if [[ -f "$config" ]]; then
            if [[ "$config" == *.yaml ]] || [[ "$config" == *.yml ]]; then
                if python3 -c "import yaml; yaml.safe_load(open('$config'))" 2>/dev/null; then
                    echo -e "${GREEN}✅ $config 语法正确${NC}"
                else
                    echo -e "${RED}❌ $config YAML 语法错误${NC}"
                    all_valid=false
                fi
            else
                if [[ -x "$config" ]] || [[ "$config" == *.sh ]]; then
                    if bash -n "$config" 2>/dev/null; then
                        echo -e "${GREEN}✅ $config 语法正确${NC}"
                    else
                        echo -e "${RED}❌ $config 语法错误${NC}"
                        all_valid=false
                    fi
                else
                    echo -e "${GREEN}✅ $config 存在${NC}"
                fi
            fi
        else
            echo -e "${YELLOW}⚠️  $config 未找到${NC}"
        fi
    done
    
    if [[ "$all_valid" == true ]]; then
        echo -e "${GREEN}✅ 所有配置文件验证通过${NC}"
    else
        echo -e "${RED}❌ 部分配置文件存在错误${NC}"
        exit 1
    fi
}

# 显示帮助信息
show_help() {
    cat << EOF
Confers CI/CD 设置脚本

使用方法: $0 [选项]

选项:
    --install     安装所有必要的工具
    --update      更新所有工具到最新版本
    --check       运行快速检查
    --full        运行完整检查
    --verify      验证配置文件
    --hooks       设置 pre-commit hooks
    --help        显示此帮助信息

示例:
    $0 --install           # 安装所有工具
    $0 --hooks             # 设置 pre-commit hooks
    $0 --check             # 运行快速检查
    $0 --full              # 运行完整检查

注意事项:
    - 安装过程可能需要几分钟时间
    - 确保网络连接正常
    - 需要足够的磁盘空间 (约 500MB)

EOF
}

# 主函数
main() {
    local install=false
    local update=false
    local check=false
    local full=false
    local verify=false
    local hooks=false
    
    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install)
                install=true
                shift
                ;;
            --update)
                update=true
                shift
                ;;
            --check)
                check=true
                shift
                ;;
            --full)
                full=true
                shift
                ;;
            --verify)
                verify=true
                shift
                ;;
            --hooks)
                hooks=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}❌ 未知参数: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
    
    check_project_root
    
    if [[ "$install" == true ]]; then
        install_rust_tools
        install_python_tools
    fi
    
    if [[ "$hooks" == true ]]; then
        setup_pre_commit
    fi
    
    if [[ "$check" == true ]]; then
        run_quick_check
    fi
    
    if [[ "$full" == true ]]; then
        run_full_check
    fi
    
    if [[ "$verify" == true ]]; then
        verify_config
    fi
    
    if [[ "$install" == false && "$hooks" == false && "$check" == false && "$full" == false && "$verify" == false ]]; then
        show_help
    fi
    
    echo -e "\n${GREEN}✨ CI/CD 设置完成！${NC}"
}

main "$@"