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
#   --skip-init       跳过 auth9-init Job（默认会先运行一次 init/migration）

set -e

# Configuration
NAMESPACE="${NAMESPACE:-auth9}"
COMPONENT="all"
WAIT="true"
DRY_RUN=""
SKIP_VALIDATION=""
RUN_INIT="true"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
K8S_DIR="$SCRIPT_DIR/k8s"

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

deprecated_keycloak_resources() {
    kubectl get deploy,svc,cm,secret,hpa,sts,pvc -n "$NAMESPACE" -o name 2>/dev/null | \
        rg '(^|/)(keycloak($|[-]))|keycloak-' || true
}

warn_deprecated_keycloak_resources() {
    local resources
    resources="$(deprecated_keycloak_resources)"
    if [ -z "$resources" ]; then
        return 0
    fi

    print_warning "检测到旧版 Keycloak 资源仍存在；升级流程不会自动清理这些遗留对象："
    echo "$resources" | sed 's/^/    /'
    print_info "确认迁移稳定后请手动删除这些资源，避免继续消耗容量或干扰排障"
}

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
            --skip-validation)
                SKIP_VALIDATION="true"
                shift
                ;;
            --skip-init)
                RUN_INIT=""
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
                echo "  --skip-validation 跳过 ConfigMap 占位符检查"
                echo "  --skip-init       跳过 auth9-init Job"
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

cfg() {
    local key="$1"
    kubectl get configmap auth9-config -n "$NAMESPACE" -o "jsonpath={.data.$key}" 2>/dev/null || true
}

validate_config() {
    if [ -n "$SKIP_VALIDATION" ]; then
        print_warning "跳过 ConfigMap 占位符检查 (--skip-validation)"
        return 0
    fi

    print_info "检查 ConfigMap 是否包含 example.com 占位符..."

    local has_placeholder=""
    local fields=(JWT_ISSUER WEBAUTHN_RP_ID CORS_ALLOWED_ORIGINS APP_BASE_URL AUTH9_CORE_PUBLIC_URL AUTH9_PORTAL_URL)

    for field in "${fields[@]}"; do
        local value="$(cfg "$field")"
        if [[ "$value" == *"example.com"* ]]; then
            print_error "$field 仍是示例域名: $value"
            has_placeholder="true"
        fi
    done

    if [ -n "$has_placeholder" ]; then
        echo ""
        print_error "ConfigMap 包含未替换的 example.com 占位符，中止升级"
        print_info "请先修改 deploy/k8s/configmap.yaml 中标记为 'REQUIRED: replace before deploy' 的字段"
        print_info "或使用 --skip-validation 跳过此检查"
        exit 1
    fi

    print_success "ConfigMap 无 example.com 占位符"
}

run_init_job() {
    print_info "运行 auth9-init（migrations / seed）..."

    if [ -n "$DRY_RUN" ]; then
        run_cmd kubectl apply -f "$K8S_DIR/auth9-core/init-job.yaml" -n "$NAMESPACE"
        print_success "auth9-init 预演完成"
        return 0
    fi

    kubectl delete job auth9-init -n "$NAMESPACE" --ignore-not-found=true >/dev/null 2>&1 || true
    kubectl apply -f "$K8S_DIR/auth9-core/init-job.yaml"

    if [ -n "$WAIT" ]; then
        print_info "等待 auth9-init 完成..."
        if kubectl wait --for=condition=complete job/auth9-init -n "$NAMESPACE" --timeout=300s >/dev/null; then
            print_success "auth9-init 已完成"
        else
            print_error "auth9-init 超时或失败"
            kubectl logs job/auth9-init -n "$NAMESPACE" --tail=100 2>/dev/null || true
            return 1
        fi
    else
        print_success "auth9-init 已触发"
    fi
}

apply_component_manifests() {
    local name="$1"
    shift
    local manifests=("$@")

    print_info "同步 $name Kubernetes 清单..."

    local manifest
    for manifest in "${manifests[@]}"; do
        run_cmd kubectl apply -f "$manifest" -n "$NAMESPACE"
    done

    if [ -n "$DRY_RUN" ]; then
        print_success "$name 清单预演完成"
    else
        print_success "$name 清单已同步"
    fi
}

upgrade_component() {
    local name="$1"
    local deployment="$2"
    shift 2
    local manifests=("$@")

    print_info "升级 $name..."

    apply_component_manifests "$name" "${manifests[@]}"

    # Get current image
    local current_image=$(kubectl get deployment "$deployment" -n "$NAMESPACE" -o jsonpath='{.spec.template.spec.containers[0].image}' 2>/dev/null || echo "unknown")
    local current_pull_policy=$(kubectl get deployment "$deployment" -n "$NAMESPACE" -o jsonpath='{.spec.template.spec.containers[0].imagePullPolicy}' 2>/dev/null || echo "unknown")
    echo "    当前镜像: $current_image"
    echo "    拉取策略: $current_pull_policy"

    if [[ "$current_image" == *":latest" && "$current_pull_policy" != "Always" ]]; then
        print_warning "$name 使用 :latest 但 imagePullPolicy=$current_pull_policy；这会导致节点复用旧镜像缓存"
    fi

    # Restart deployment after reconciling manifests so pods re-pull the intended image
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

    # Validate ConfigMap before upgrade
    validate_config
    warn_deprecated_keycloak_resources

    if [ -n "$RUN_INIT" ]; then
        run_init_job
    else
        print_warning "跳过 auth9-init (--skip-init)"
    fi

    # Upgrade components
    case "$COMPONENT" in
        core)
            upgrade_component \
                "auth9-core" \
                "auth9-core" \
                "$K8S_DIR/auth9-core/service.yaml" \
                "$K8S_DIR/auth9-core/deployment.yaml" \
                "$K8S_DIR/auth9-core/hpa.yaml"
            ;;
        portal)
            upgrade_component \
                "auth9-portal" \
                "auth9-portal" \
                "$K8S_DIR/auth9-portal/service.yaml" \
                "$K8S_DIR/auth9-portal/deployment.yaml" \
                "$K8S_DIR/auth9-portal/hpa.yaml"
            ;;
        all)
            upgrade_component \
                "auth9-core" \
                "auth9-core" \
                "$K8S_DIR/auth9-core/service.yaml" \
                "$K8S_DIR/auth9-core/deployment.yaml" \
                "$K8S_DIR/auth9-core/hpa.yaml"
            upgrade_component \
                "auth9-portal" \
                "auth9-portal" \
                "$K8S_DIR/auth9-portal/service.yaml" \
                "$K8S_DIR/auth9-portal/deployment.yaml" \
                "$K8S_DIR/auth9-portal/hpa.yaml"
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
