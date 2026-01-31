#!/usr/bin/env zsh
# Auth9 Cleanup Script
#
# This script cleans up Auth9 resources from the Kubernetes cluster.
#
# Usage:
#   ./cleanup.sh [options]
#
# Options:
#   --namespace NS       Use a different namespace (default: auth9)
#   --dry-run            Show what would be deleted without executing

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
            * ) echo "Please answer yes or no." ;;
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
                echo "Usage: $0 [options]"
                echo ""
                echo "Options:"
                echo "  --namespace NS       Use a different namespace (default: auth9)"
                echo "  --dry-run            Show what would be deleted without executing"
                echo "  -h, --help           Show this help"
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
}

check_namespace() {
    if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
        print_warning "Namespace '$NAMESPACE' does not exist"
        exit 0
    fi
}

show_resources() {
    echo -e "${BOLD}Current resources in namespace '$NAMESPACE':${NC}"
    echo ""

    echo -e "${YELLOW}Deployments:${NC}"
    kubectl get deployments -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  (none)"
    echo ""

    echo -e "${YELLOW}StatefulSets:${NC}"
    kubectl get statefulsets -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  (none)"
    echo ""

    echo -e "${YELLOW}Jobs:${NC}"
    kubectl get jobs -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  (none)"
    echo ""

    echo -e "${YELLOW}Services:${NC}"
    kubectl get services -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  (none)"
    echo ""

    echo -e "${YELLOW}Secrets:${NC}"
    kubectl get secrets -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "default-token\|service-account" || echo "  (none)"
    echo ""

    echo -e "${YELLOW}ConfigMaps:${NC}"
    kubectl get configmaps -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "kube-root-ca" || echo "  (none)"
    echo ""

    echo -e "${YELLOW}PVCs (Database Data):${NC}"
    kubectl get pvc -n "$NAMESPACE" --no-headers 2>/dev/null || echo "  (none)"
    echo ""
}

delete_jobs() {
    local job_count=$(kubectl get jobs -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$job_count" -eq 0 ]; then
        print_info "No jobs to delete"
        return
    fi

    print_info "Deleting $job_count job(s)..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get jobs -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete jobs --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "Jobs deleted"
    fi
}

delete_deployments() {
    local deploy_count=$(kubectl get deployments -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$deploy_count" -eq 0 ]; then
        print_info "No deployments to delete"
        return
    fi

    print_info "Deleting $deploy_count deployment(s)..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get deployments -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete deployments --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "Deployments deleted"
    fi
}

delete_statefulsets() {
    local sts_count=$(kubectl get statefulsets -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$sts_count" -eq 0 ]; then
        print_info "No statefulsets to delete"
        return
    fi

    print_info "Deleting $sts_count statefulset(s)..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get statefulsets -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete statefulsets --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "StatefulSets deleted"
    fi
}

delete_services() {
    local svc_count=$(kubectl get services -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$svc_count" -eq 0 ]; then
        print_info "No services to delete"
        return
    fi

    print_info "Deleting $svc_count service(s)..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get services -n "$NAMESPACE" -o name 2>/dev/null || true
    else
        kubectl delete services --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "Services deleted"
    fi
}

delete_configmaps() {
    local cm_count=$(kubectl get configmaps -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "kube-root-ca" | wc -l | tr -d ' ')
    if [ "$cm_count" -eq 0 ]; then
        print_info "No configmaps to delete"
        return
    fi

    print_info "Deleting $cm_count configmap(s)..."
    if [ -n "$DRY_RUN" ]; then
        kubectl get configmaps -n "$NAMESPACE" -o name 2>/dev/null | grep -v "kube-root-ca" || true
    else
        kubectl delete configmap auth9-config -n "$NAMESPACE" --ignore-not-found=true
        print_success "ConfigMaps deleted"
    fi
}

interactive_delete_secrets() {
    print_progress "5/7" "Secrets"

    echo ""
    echo -e "  ${YELLOW}Current secrets:${NC}"
    kubectl get secrets -n "$NAMESPACE" --no-headers 2>/dev/null | grep -v "default-token\|service-account" | while read line; do
        echo "    $line"
    done || echo "    (none)"
    echo ""

    print_warning "Secrets contain sensitive data:"
    echo "    - DATABASE_URL (database connection string)"
    echo "    - REDIS_URL"
    echo "    - JWT_SECRET, SESSION_SECRET"
    echo "    - KEYCLOAK_ADMIN_PASSWORD"
    echo "    - KEYCLOAK_ADMIN_CLIENT_SECRET"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[Dry Run] Would ask to delete secrets"
        return
    fi

    if confirm_action "  Delete secrets? (You will need to reconfigure on next deploy)"; then
        print_info "Deleting secrets..."
        kubectl delete secret auth9-secrets -n "$NAMESPACE" --ignore-not-found=true
        kubectl delete secret keycloak-secrets -n "$NAMESPACE" --ignore-not-found=true
        kubectl delete secret ghcr-secret -n "$NAMESPACE" --ignore-not-found=true
        print_success "Secrets deleted"
    else
        print_info "Keeping secrets"
    fi
}

interactive_delete_pvcs() {
    print_progress "6/7" "Persistent Volume Claims (Database Data)"

    local pvc_count=$(kubectl get pvc -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')
    if [ "$pvc_count" -eq 0 ]; then
        print_info "No PVCs to delete"
        return
    fi

    echo ""
    echo -e "  ${YELLOW}Current PVCs:${NC}"
    kubectl get pvc -n "$NAMESPACE" --no-headers 2>/dev/null | while read line; do
        echo "    $line"
    done
    echo ""

    print_warning "PVCs contain database data:"
    echo "    - Keycloak PostgreSQL data"
    echo "    - User accounts, realm configuration, etc."
    echo ""
    echo -e "  ${RED}WARNING: Deleting PVCs will permanently destroy all database data!${NC}"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[Dry Run] Would ask to delete PVCs"
        return
    fi

    if confirm_action "  Delete PVCs? (THIS WILL DESTROY ALL DATABASE DATA)"; then
        print_info "Deleting PVCs..."
        kubectl delete pvc --all -n "$NAMESPACE" --ignore-not-found=true
        print_success "PVCs deleted"
    else
        print_info "Keeping PVCs (database data preserved)"
    fi
}

interactive_delete_namespace() {
    print_progress "7/7" "Namespace"

    echo ""
    print_info "Deleting namespace will remove any remaining resources"
    echo ""

    if [ -n "$DRY_RUN" ]; then
        print_info "[Dry Run] Would delete namespace '$NAMESPACE'"
        return
    fi

    if confirm_action "  Delete namespace '$NAMESPACE'?"; then
        print_info "Deleting namespace..."
        kubectl delete namespace "$NAMESPACE" --ignore-not-found=true
        print_success "Namespace deleted"
    else
        print_info "Keeping namespace"
    fi
}

main() {
    parse_arguments "$@"

    print_header "Auth9 Cleanup"

    echo -e "${YELLOW}Namespace:${NC} $NAMESPACE"
    echo -e "${YELLOW}Mode:${NC} $([ -n "$DRY_RUN" ] && echo "Dry Run (no changes)" || echo "Interactive")"
    echo ""

    check_namespace
    show_resources

    if [ -n "$DRY_RUN" ]; then
        print_warning "Dry run mode - showing what would happen"
    fi

    if ! confirm_action "Start cleanup process?"; then
        print_info "Cleanup cancelled"
        exit 0
    fi

    print_header "Cleaning up resources"

    # Step 1-4: Delete workloads (no confirmation needed)
    print_progress "1/7" "Jobs"
    delete_jobs

    print_progress "2/7" "Deployments"
    delete_deployments

    print_progress "3/7" "StatefulSets"
    delete_statefulsets

    print_progress "4/7" "Services & ConfigMaps"
    delete_services
    delete_configmaps

    # Step 5-6: Interactive confirmation for sensitive data
    interactive_delete_secrets
    interactive_delete_pvcs

    # Step 7: Namespace (only if --all)
    interactive_delete_namespace

    print_header "Cleanup Complete"

    if [ -z "$DRY_RUN" ]; then
        echo -e "${YELLOW}Remaining resources in '$NAMESPACE':${NC}"
        if kubectl get namespace "$NAMESPACE" &>/dev/null; then
            kubectl get all,secrets,configmaps,pvc -n "$NAMESPACE" 2>/dev/null || echo "  (namespace deleted)"
        else
            echo "  Namespace has been deleted"
        fi
    fi

    echo ""
    print_info "To redeploy, run: ./deploy/deploy.sh"
}

main "$@"
