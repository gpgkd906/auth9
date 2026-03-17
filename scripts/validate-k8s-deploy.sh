#!/usr/bin/env zsh
# Validate Auth9 Kubernetes deployment and critical runtime wiring.

set -euo pipefail

NAMESPACE="${NAMESPACE:-auth9}"
STRICT="false"
SKIP_PUBLIC="false"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

FAILURES=0
WARNINGS=0

print_success() { echo -e "  ${GREEN}✓${NC} $1"; }
print_error() { echo -e "  ${RED}✗${NC} $1"; FAILURES=$((FAILURES + 1)); }
print_warning() { echo -e "  ${YELLOW}⚠${NC} $1"; WARNINGS=$((WARNINGS + 1)); }
print_info() { echo -e "  ${CYAN}ℹ${NC} $1"; }

usage() {
    cat <<EOF
用法: ./scripts/validate-k8s-deploy.sh [选项]

选项:
  --namespace NS   指定命名空间（默认: auth9）
  --strict         将 warning 也视为失败
  --skip-public    跳过公网 URL curl 检查
  -h, --help       显示帮助
EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            --strict)
                STRICT="true"
                shift
                ;;
            --skip-public)
                SKIP_PUBLIC="true"
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                echo "未知参数: $1" >&2
                usage
                exit 1
                ;;
        esac
    done
}

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "缺少命令: $1" >&2
        exit 1
    fi
}

cfg() {
    local cm="$1"
    local key="$2"
    kubectl get configmap "$cm" -n "$NAMESPACE" -o "jsonpath={.data.$key}" 2>/dev/null || true
}

secret_exists() {
    local name="$1"
    local key="$2"
    local raw
    raw=$(kubectl get secret "$name" -n "$NAMESPACE" -o "jsonpath={.data.$key}" 2>/dev/null || true)
    [[ -n "$raw" ]]
}

check_equals() {
    local label="$1"
    local actual="$2"
    local expected="$3"
    if [[ "$actual" == "$expected" && -n "$actual" ]]; then
        print_success "$label = $actual"
    else
        print_error "$label 不匹配: actual='$actual' expected='$expected'"
    fi
}

check_nonempty() {
    local label="$1"
    local value="$2"
    if [[ -n "$value" ]]; then
        print_success "$label 已配置"
    else
        print_error "$label 为空"
    fi
}

check_not_example_domain() {
    local label="$1"
    local value="$2"
    if [[ "$value" == *"example.com"* || "$value" == *"auth9.example.com"* ]]; then
        print_error "$label 仍是示例域名: $value"
    else
        print_success "$label 不是示例域名"
    fi
}

check_csv_contains() {
    local label="$1"
    local csv="$2"
    local expected="$3"
    local normalized_csv=",$(echo "$csv" | tr -d ' '),"
    local normalized_expected=",$expected,"

    if [[ "$normalized_csv" == *"$normalized_expected"* ]]; then
        print_success "$label 包含 $expected"
    else
        print_error "$label 缺少 $expected: $csv"
    fi
}

check_rollout() {
    local kind="$1"
    local name="$2"
    print_info "检查 $kind/$name rollout"
    if kubectl rollout status "$kind/$name" -n "$NAMESPACE" --timeout=20s >/dev/null 2>&1; then
        print_success "$kind/$name 已就绪"
    else
        print_error "$kind/$name 未就绪"
    fi
}

check_http() {
    local label="$1"
    local url="$2"
    local pattern="${3:-}"
    if [[ "$SKIP_PUBLIC" == "true" ]]; then
        print_info "跳过公网检查: $label"
        return 0
    fi

    local output
    if ! output=$(curl -sSIL --max-redirs 5 --connect-timeout 10 "$url" 2>/dev/null); then
        print_error "$label 不可达: $url"
        return 0
    fi

    if [[ -n "$pattern" ]] && ! echo "$output" | rg -q "$pattern"; then
        print_error "$label 响应未匹配预期 ($pattern): $url"
        return 0
    fi

    print_success "$label 可达: $url"
}

main() {
    parse_args "$@"
    require_cmd kubectl
    require_cmd curl
    require_cmd rg

    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║    Auth9 K8s 部署校验                      ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════╝${NC}"
    echo ""
    print_info "命名空间: $NAMESPACE"

    if ! kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
        echo "命名空间不存在: $NAMESPACE" >&2
        exit 1
    fi

    local app_base_url auth9_core_public_url auth9_portal_url keycloak_public_url jwt_issuer cors_allowed_origins
    local keycloak_auth9_api_url keycloak_hostname tracing_enabled tracing_endpoint
    local portal_envfrom_secret portal_session_secret_ref

    app_base_url="$(cfg auth9-config APP_BASE_URL)"
    auth9_core_public_url="$(cfg auth9-config AUTH9_CORE_PUBLIC_URL)"
    auth9_portal_url="$(cfg auth9-config AUTH9_PORTAL_URL)"
    keycloak_public_url="$(cfg auth9-config KEYCLOAK_PUBLIC_URL)"
    jwt_issuer="$(cfg auth9-config JWT_ISSUER)"
    cors_allowed_origins="$(cfg auth9-config CORS_ALLOWED_ORIGINS)"
    tracing_enabled="$(cfg auth9-config OTEL_TRACING_ENABLED)"
    tracing_endpoint="$(cfg auth9-config OTEL_EXPORTER_OTLP_ENDPOINT)"

    keycloak_auth9_api_url="$(cfg keycloak-config AUTH9_API_URL)"
    keycloak_hostname="$(cfg keycloak-config KC_HOSTNAME)"

    check_rollout deploy auth9-core
    check_rollout deploy auth9-portal
    check_rollout deploy keycloak

    if kubectl get job/auth9-init -n "$NAMESPACE" >/dev/null 2>&1; then
        if kubectl wait --for=condition=complete job/auth9-init -n "$NAMESPACE" --timeout=5s >/dev/null 2>&1; then
            print_success "job/auth9-init 已完成"
        else
            print_warning "job/auth9-init 存在但未处于 Complete 状态"
        fi
    else
        print_info "job/auth9-init 已被 TTL 清理，跳过状态检查"
    fi

    echo ""
    print_info "检查关键配置一致性"
    check_nonempty "APP_BASE_URL" "$app_base_url"
    check_nonempty "AUTH9_CORE_PUBLIC_URL" "$auth9_core_public_url"
    check_nonempty "AUTH9_PORTAL_URL" "$auth9_portal_url"
    check_nonempty "KEYCLOAK_PUBLIC_URL" "$keycloak_public_url"
    check_nonempty "CORS_ALLOWED_ORIGINS" "$cors_allowed_origins"
    check_equals "APP_BASE_URL" "$app_base_url" "$auth9_portal_url"
    check_equals "JWT_ISSUER" "$jwt_issuer" "$auth9_core_public_url"
    check_equals "keycloak-config.AUTH9_API_URL" "$keycloak_auth9_api_url" "$auth9_core_public_url"
    check_equals "keycloak-config.KC_HOSTNAME" "$keycloak_hostname" "$keycloak_public_url"
    check_csv_contains "CORS_ALLOWED_ORIGINS" "$cors_allowed_origins" "$auth9_portal_url"
    check_csv_contains "CORS_ALLOWED_ORIGINS" "$cors_allowed_origins" "$keycloak_public_url"
    check_not_example_domain "APP_BASE_URL" "$app_base_url"
    check_not_example_domain "AUTH9_CORE_PUBLIC_URL" "$auth9_core_public_url"
    check_not_example_domain "AUTH9_PORTAL_URL" "$auth9_portal_url"
    check_not_example_domain "KEYCLOAK_PUBLIC_URL" "$keycloak_public_url"

    if [[ "$tracing_enabled" == "true" ]]; then
        check_nonempty "OTEL_EXPORTER_OTLP_ENDPOINT" "$tracing_endpoint"
    else
        print_success "OTEL_TRACING_ENABLED=false"
    fi

    echo ""
    print_info "检查 secrets 与 deployment wiring"
    if secret_exists auth9-secrets AUTH9_ADMIN_PASSWORD; then
        print_success "auth9-secrets.AUTH9_ADMIN_PASSWORD 已存在"
    else
        print_error "auth9-secrets.AUTH9_ADMIN_PASSWORD 缺失"
    fi

    portal_envfrom_secret="$(kubectl get deploy auth9-portal -n "$NAMESPACE" -o jsonpath='{.spec.template.spec.containers[0].envFrom[*].secretRef.name}' 2>/dev/null || true)"
    if [[ -z "$portal_envfrom_secret" ]]; then
        print_success "auth9-portal 未通过 envFrom 注入整包 secret"
    else
        print_error "auth9-portal 仍通过 envFrom 注入 secret: $portal_envfrom_secret"
    fi

    portal_session_secret_ref="$(kubectl get deploy auth9-portal -n "$NAMESPACE" -o jsonpath='{.spec.template.spec.containers[0].env[?(@.name=="SESSION_SECRET")].valueFrom.secretKeyRef.name}' 2>/dev/null || true)"
    if [[ "$portal_session_secret_ref" == "auth9-secrets" ]]; then # pragma: allowlist secret
        print_success "auth9-portal 仅按需引用 SESSION_SECRET"
    else
        print_error "auth9-portal 未正确引用 SESSION_SECRET"
    fi

    echo ""
    print_info "检查公网可达性"
    check_http "Portal /login" "$auth9_portal_url/login" "HTTP/[12](\\.[0-9])? 200"
    check_http "Core /health" "$auth9_core_public_url/health" "HTTP/[12](\\.[0-9])? 200"
    check_http "Keycloak realm" "$keycloak_public_url/realms/auth9" "HTTP/[12](\\.[0-9])? 200"
    check_http "Public branding" "$auth9_core_public_url/api/v1/public/branding?client_id=auth9-portal" "HTTP/[12](\\.[0-9])? 200"

    echo ""
    if [[ "$FAILURES" -eq 0 && ("$STRICT" != "true" || "$WARNINGS" -eq 0) ]]; then
        echo -e "${GREEN}${BOLD}部署校验通过${NC}"
        [[ "$WARNINGS" -gt 0 ]] && echo -e "${YELLOW}warnings: $WARNINGS${NC}"
        exit 0
    fi

    echo -e "${RED}${BOLD}部署校验失败${NC}"
    echo -e "failures: $FAILURES"
    echo -e "warnings: $WARNINGS"
    exit 1
}

main "$@"
