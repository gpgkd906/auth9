#!/usr/bin/env zsh
# Auth9 清理脚本
#
# 本脚本用于从 Kubernetes 集群中清理 Auth9 资源。
#
# 用法:
#   ./cleanup.sh [选项]
#
# 选项:
#   --namespace NS       使用其他命名空间（默认: auth9）
#   --dry-run            仅显示将要删除的内容，不实际执行

set -e

# Configuration
NAMESPACE="${NAMESPACE:-auth9}"
DRY_RUN=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

print_header() {
    local title="$1"
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    printf "${BLUE}║${NC} %-42s ${BLUE}║${NC}\n" "$title"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo ""
}

print_success() {
    echo -e "  ${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "  ${RED}✗${NC} $1"
}

print_warning() {
    echo -e "  ${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "  ${CYAN}ℹ${NC} $1"
}

print_progress() {
    local step="$1"
    local message="$2"
    echo ""
    echo -e "${GREEN}[$step]${NC} ${BOLD}$message${NC}"
}

confirm_action() {
    local message="$1"
    local response

    while true; do
        read "response?$message [y/N]: "
        case "$response" in
            [Yy]* ) return 0 ;;
            [Nn]* | "" ) return 1 ;;
            * ) echo "请回答 yes 或 no。" ;;
        esac
    done
}

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            -h|--help)
                echo "用法: $0 [选项]"
                echo ""
                echo "选项:"
                echo "  --namespace NS       使用其他命名空间（默认: auth9）"
                echo "  --dry-run            仅显示将要删除的内容，不实际执行"
                echo "  -h, --help           显示帮助信息"
                exit 0
                ;;
            *)
                echo -e "${RED}未知选项: $1${NC}"
                echo "使用 --help 查看用法信息"
                exit 1
                ;;
        esac
    done
}

check_namespace() {
    if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
        print_warning "命名空间 '$NAMESPACE' 不存在"
        exit 0
    fi
}

show_resources() {
    echo -e "${BOLD}命名空间 '$NAMESPACE' 中的当前资源:${NC}"
    echo ""

    echo -e "${YELLOW}Deployments:${NC}"
    kubectl get deployments -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  （无）"
    echo ""

    echo -e "${YELLOW}StatefulSets:${NC}"
    kubectl get statefulsets -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  （无）"
    echo ""

    echo -e "${YELLOW}Jobs:${NC}"
    kubectl get jobs -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  （无）"
    echo ""

    echo -e "${YELLOW}HorizontalPodAutoscalers:${NC}"
    kubectl get hpa -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  （无）"
    echo ""

    echo -e "${YELLOW}Services:${NC}"
    kubectl get services -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  （无）"
    echo ""

    echo -e "${YELLOW}Secrets:${NC}"
    kubectl get secrets -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "default-token\|service-account" || echo "  （无）"
    echo ""

    echo -e "${YELLOW}ConfigMaps:${NC}"
    kubectl get configmaps -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "kube-root-ca" || echo "  （无）"
    echo ""

    echo -e "${YELLOW}PVCs（数据库数据）:${NC}"
    kubectl get pvc -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  （无）"
    echo ""
}

delete_jobs() {
    local job_count=$(kubectl get jobs -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$job_count" -eq 0 ]; then
        print_info "没有 Job 需要删除"
        return
    fi

    print_info "正在删除 $job_count 个 Job..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get jobs -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete jobs --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "Jobs 已删除"
    fi
}

delete_deployments() {
    local deploy_count=$(kubectl get deployments -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$deploy_count" -eq 0 ]; then
        print_info "没有 Deployment 需要删除"
        return
    fi

    print_info "正在删除 $deploy_count 个 Deployment..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get deployments -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete deployments --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "Deployments 已删除"
    fi
}

delete_statefulsets() {
    local sts_count=$(kubectl get statefulsets -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$sts_count" -eq 0 ]; then
        print_info "没有 StatefulSet 需要删除"
        return
    fi

    print_info "正在删除 $sts_count 个 StatefulSet..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get statefulsets -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete statefulsets --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "StatefulSets 已删除"
    fi
}

delete_services() {
    local svc_count=$(kubectl get services -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$svc_count" -eq 0 ]; then
        print_info "没有 Service 需要删除"
        return
    fi

    print_info "正在删除 $svc_count 个 Service..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get services -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete services --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "Services 已删除"
    fi
}

delete_hpas() {
    local hpa_count=$(kubectl get hpa -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$hpa_count" -eq 0 ]; then
        print_info "没有 HPA 需要删除"
        return
    fi

    print_info "正在删除 $hpa_count 个 HPA..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get hpa -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete hpa --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "HPAs 已删除"
    fi
}

delete_configmaps() {
    local cm_count=$(kubectl get configmaps -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "kube-root-ca" | wc -l | tr -d ' ')
    if [ "$cm_count" -eq 0 ]; then
        print_info "没有 ConfigMap 需要删除"
        return
    fi

    print_info "正在删除 $cm_count 个 ConfigMap..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get configmaps -n "$NAMESPACE" -o name 2>/dev/null | grep -v "kube-root-ca" || true
    else
        kubectl delete configmap auth9-config -n "$NAMESPACE" --ignore-not-found=true
        print_success "ConfigMaps 已删除"
    fi
}

reset_tidb_database() {
    print_progress "6/9" "重置 TiDB 数据库"

    echo ""
    print_warning "此操作将删除 auth9 数据库中的所有数据！"
    print_warning "KEYCLOAK_ADMIN_CLIENT_SECRET 将被删除（下次部署重新生成）"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[预演] 将询问是否重置数据库"
        return 1
    fi

    if ! confirm_action "  确定要重置数据库吗？"; then
        print_info "跳过数据库重置"
        return 1
    fi

    # 检查 secret 是否存在（需要从中获取 DATABASE_URL）
    if ! kubectl get secret auth9-secrets -n "$NAMESPACE" &>/dev/null; then
        print_error "auth9-secrets 不存在，无法获取数据库连接信息"
        return 1
    fi

    print_info "正在运行数据库重置..."

    # 删除旧的 reset job（如果存在）
    kubectl delete job auth9-reset -n "$NAMESPACE" --ignore-not-found=true 2>/dev/null

    # 创建 reset job
    cat <<EOF | kubectl apply -f -
apiVersion: batch/v1
kind: Job
metadata:
  name: auth9-reset
  namespace: $NAMESPACE
spec:
  ttlSecondsAfterFinished: 60
  template:
    spec:
      restartPolicy: Never
      imagePullSecrets:
        - name: ghcr-secret
      containers:
        - name: auth9-reset
          image: ghcr.io/gpgkd906/auth9-core:latest
          command: ["auth9-core", "reset"]
          envFrom:
            - secretRef:
                name: auth9-secrets
EOF

    # 等待 Job 完成
    print_info "等待数据库重置完成（超时: 120秒）..."
    if kubectl wait --for=condition=complete job/auth9-reset -n "$NAMESPACE" --timeout=120s 2>/dev/null; then
        # 显示日志
        echo ""
        kubectl logs job/auth9-reset -n "$NAMESPACE" 2>/dev/null || true
        echo ""

        # 清理 job
        kubectl delete job auth9-reset -n "$NAMESPACE" --ignore-not-found=true 2>/dev/null
        print_success "数据库已重置"

        # 只删除 KEYCLOAK_ADMIN_CLIENT_SECRET（需要重新生成），保留其他配置
        print_info "正在删除 KEYCLOAK_ADMIN_CLIENT_SECRET..."
        kubectl patch secret auth9-secrets -n "$NAMESPACE" \
            --type='json' \
            -p='[{"op": "remove", "path": "/data/KEYCLOAK_ADMIN_CLIENT_SECRET"}]' 2>/dev/null || true
        print_success "KEYCLOAK_ADMIN_CLIENT_SECRET 已删除（下次部署将重新生成）"

        return 0
    else
        print_error "数据库重置失败或超时"
        echo ""
        echo "  查看日志: kubectl logs job/auth9-reset -n $NAMESPACE"
        kubectl logs job/auth9-reset -n "$NAMESPACE" --tail=10 2>/dev/null || true
        return 1
    fi
}

interactive_delete_secrets() {
    print_progress "7/9" "Secrets"

    # 如果数据库重置成功，提示 client secret 已删除
    if [ -n "$DB_RESET_DONE" ]; then
        print_info "KEYCLOAK_ADMIN_CLIENT_SECRET 已在数据库重置步骤中删除"
        echo ""
    fi

    echo ""
    echo -e "  ${YELLOW}当前密钥:${NC}"
    kubectl get secrets -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "default-token\|service-account" | while read line; do
        echo "    $line"
    done || echo "    （无）"
    echo ""

    print_warning "密钥包含敏感数据:"
    echo "    - DATABASE_URL（数据库连接字符串）"
    echo "    - REDIS_URL"
    echo "    - JWT_SECRET、SESSION_SECRET"
    echo "    - KEYCLOAK_ADMIN_PASSWORD"
    echo "    - KEYCLOAK_ADMIN_CLIENT_SECRET"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[预演] 将询问是否删除密钥"
        return
    fi

    if confirm_action "  删除剩余密钥？（下次部署需要重新配置）"; then
        print_info "正在删除密钥..."
        kubectl delete secret auth9-secrets -n "$NAMESPACE" --ignore-not-found=true
        kubectl delete secret keycloak-secrets -n "$NAMESPACE" --ignore-not-found=true
        kubectl delete secret ghcr-secret -n "$NAMESPACE" --ignore-not-found=true
        print_success "密钥已删除"
    else
        print_info "保留密钥"
    fi
}

interactive_delete_pvcs() {
    print_progress "8/9" "持久卷声明（数据库数据）"

    local pvc_count=$(kubectl get pvc -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$pvc_count" -eq 0 ]; then
        print_info "没有 PVC 需要删除"
        return
    fi

    echo ""
    echo -e "  ${YELLOW}当前 PVCs:${NC}"
    kubectl get pvc -n "$NAMESPACE" --no-headers 2>/dev/null | while read line; do
        echo "    $line"
    done
    echo ""

    print_warning "PVCs 包含数据库数据:"
    echo "    - Keycloak PostgreSQL 数据"
    echo "    - 用户账户、Realm 配置等"
    echo ""
    echo -e "  ${RED}警告: 删除 PVCs 将永久销毁所有数据库数据！${NC}"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[预演] 将询问是否删除 PVCs"
        return
    fi

    if confirm_action "  删除 PVCs？（这将销毁所有数据库数据）"; then
        print_info "正在删除 PVCs..."
        kubectl delete pvc --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "PVCs 已删除"
    else
        print_info "保留 PVCs（数据库数据已保留）"
    fi
}

interactive_delete_namespace() {
    print_progress "9/9" "命名空间"

    echo ""
    print_info "删除命名空间将移除所有剩余资源"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[预演] 将删除命名空间 '$NAMESPACE'"
        return
    fi

    if confirm_action "  删除命名空间 '$NAMESPACE'？"; then
        print_info "正在删除命名空间..."
        kubectl delete namespace "$NAMESPACE" --ignore-not-found=true
        print_success "命名空间已删除"
    else
        print_info "保留命名空间"
    fi
}

main() {
    parse_arguments "$@"

    print_header "Auth9 清理"

    echo -e "${YELLOW}命名空间:${NC} $NAMESPACE"
    echo -e "${YELLOW}模式:${NC} $([ -n "$DRY_RUN" ] && echo "预演模式（不做实际更改）" || echo "交互式")"
    echo ""

    check_namespace
    show_resources

    if [ -n "$DRY_RUN" ]; then
        print_warning "预演模式 - 仅显示将要执行的操作"
    fi

    if ! confirm_action "开始清理？"; then
        print_info "清理已取消"
        exit 0
    fi

    print_header "正在清理资源"

    # Step 1-5: Delete workloads (no confirmation needed)
    print_progress "1/9" "Jobs"
    delete_jobs

    print_progress "2/9" "Deployments"
    delete_deployments

    print_progress "3/9" "StatefulSets"
    delete_statefulsets

    print_progress "4/9" "HorizontalPodAutoscalers"
    delete_hpas

    print_progress "5/9" "Services 和 ConfigMaps"
    delete_services
    delete_configmaps

    # Step 6: Reset TiDB database (optional, before deleting secrets)
    # Track if database was reset (secrets deleted as part of reset)
    DB_RESET_DONE=""
    if reset_tidb_database; then
        DB_RESET_DONE="true"
    fi

    # Step 7: Interactive confirmation for sensitive data
    interactive_delete_secrets
    interactive_delete_pvcs

    # Step 8-9: Namespace
    interactive_delete_namespace

    print_header "清理完成"

    if [ -z "$DRY_RUN" ]; then
        echo -e "${YELLOW}'$NAMESPACE' 中的剩余资源:${NC}"
        if kubectl get namespace "$NAMESPACE" &>/dev/null; then
            kubectl get all,secrets,configmaps,pvc -n "$NAMESPACE" 2>/dev/null || echo "  （命名空间已删除）"
        else
            echo "  命名空间已删除"
        fi
    fi

    echo ""
    print_info "重新部署请运行: ./deploy/deploy.sh"
}

main "$@"
