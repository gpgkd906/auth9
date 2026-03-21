#!/usr/bin/env zsh
# Auth9 升级脚本
#
# 用于升级 Auth9 到最新版本（拉取最新镜像并重启）
#
# 用法:
#   ./upgrade.sh [选项]
#
# 选项:
#   --namespace NS    使用其他命名空间（默认: auth9）
#   --component NAME  只升级指定组件（core, portal, all）
#   --no-wait         不等待 rollout 完成
#   --dry-run         仅显示将要执行的命令

set -e

# Configuration
NAMESPACE="${NAMESPACE:-auth9}"
COMPONENT="all"
WAIT="true"
DRY_RUN=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

print_success() { echo -e "  ${GREEN}✓${NC} $1"; }
print_error() { echo -e "  ${RED}✗${NC} $1"; }
print_info() { echo -e "  ${CYAN}ℹ${NC} $1"; }
print_warning() { echo -e "  ${YELLOW}⚠${NC} $1"; }

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            --component)
                COMPONENT="$2"
                shift 2
                ;;
            --no-wait)
                WAIT=""
                shift
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            -h|--help)
                echo "用法: ./upgrade.sh [选项]"
                echo ""
                echo "选项:"
                echo "  --namespace NS    使用其他命名空间（默认: auth9）"
                echo "  --component NAME  只升级指定组件: core, portal, all（默认: all）"
                echo "  --no-wait         不等待 rollout 完成"
                echo "  --dry-run         仅显示将要执行的命令"
                echo "  -h, --help        显示帮助信息"
                exit 0
                ;;
            *)
                echo -e "${RED}未知选项: $1${NC}"
                exit 1
                ;;
        esac
    done
}

run_cmd() {
    if [ -n "$DRY_RUN" ]; then
        echo -e "  ${YELLOW}[dry-run]${NC} $*"
    else
        "$@"
    fi
}

upgrade_component() {
    local name="$1"
    local deployment="$2"

    print_info "升级 $name..."

    # Get current image
    local current_image=$(kubectl get deployment "$deployment" -n "$NAMESPACE" -o jsonpath='{.spec.template.spec.containers[0].image}' 2>/dev/null || echo "unknown")
    echo "    当前镜像: $current_image"

    # Restart deployment (triggers image pull due to imagePullPolicy: Always)
    run_cmd kubectl rollout restart deployment/"$deployment" -n "$NAMESPACE"

    if [ -n "$WAIT" ] && [ -z "$DRY_RUN" ]; then
        print_info "等待 $name rollout 完成..."
        if kubectl rollout status deployment/"$deployment" -n "$NAMESPACE" --timeout=300s; then
            print_success "$name 升级完成"
        else
            print_error "$name rollout 超时"
            return 1
        fi
    else
        print_success "$name 升级已触发"
    fi
}

main() {
    parse_arguments "$@"

    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║         Auth9 升级                          ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}命名空间:${NC} $NAMESPACE"
    echo -e "${YELLOW}组件:${NC} $COMPONENT"
    [ -n "$DRY_RUN" ] && echo -e "${YELLOW}模式:${NC} 预演（不实际执行）"
    echo ""

    # Check namespace exists
    if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
        print_error "命名空间 '$NAMESPACE' 不存在"
        exit 1
    fi

    # Upgrade components
    case "$COMPONENT" in
        core)
            upgrade_component "auth9-core" "auth9-core"
            ;;
        portal)
            upgrade_component "auth9-portal" "auth9-portal"
            ;;
        all)
            upgrade_component "auth9-core" "auth9-core"
            upgrade_component "auth9-portal" "auth9-portal"
            ;;
        *)
            print_error "未知组件: $COMPONENT（可选: core, portal, all）"
            exit 1
            ;;
    esac

    echo ""
    if [ -z "$DRY_RUN" ]; then
        echo -e "${GREEN}升级完成！${NC}"
        echo ""
        echo -e "${YELLOW}当前 Pod 状态:${NC}"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/part-of=auth9
    fi
}

main "$@"
