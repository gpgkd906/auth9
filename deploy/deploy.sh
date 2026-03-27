#!/usr/bin/env zsh
# Auth9 交互式部署脚本
#
# 本脚本将 Auth9 部署到 Kubernetes 集群，支持交互式配置。
#
# 用法:
#   ./deploy.sh [选项]
#
# 选项:
#   --interactive       启用交互模式（默认）
#   --non-interactive   禁用交互模式，使用原始行为
#   --dry-run           仅打印将要执行的操作，不实际执行
#   --namespace NS      使用其他命名空间（默认: auth9）
#   --config-file FILE  从文件加载配置（JSON 或 env 格式）
#   --with-observability    强制部署可观测性资源
#   --without-observability 跳过可观测性资源部署
#   --skip-validation   在非交互模式下跳过 ConfigMap 占位符检查
#
# 前提条件:
#   - kubectl 已配置集群访问权限
#   - openssl（用于生成密钥）
#   - base64（用于密钥编码）

set -e

# Configuration
NAMESPACE="${NAMESPACE:-auth9}"
K8S_DIR="$(cd "$(dirname "$0")" && pwd)/k8s"
DRY_RUN=""
INTERACTIVE="true"
CONFIG_FILE=""
OBSERVABILITY_MODE="auto"
SKIP_VALIDATION=""

# Associative arrays for configuration
declare -A AUTH9_SECRETS
declare -A CONFIGMAP_VALUES

# Admin credentials
AUTH9_ADMIN_USERNAME="admin"
AUTH9_ADMIN_PASSWORD=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Signal handling
trap 'print_error "安装被中断"; exit 130' INT TERM

################################################################################
# Phase 1: Basic Utility Functions
################################################################################

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

prompt_user() {
    local message="$1"
    local default="$2"
    local input

    if [ -n "$default" ]; then
        read "input?$message [$default]: "
        echo "${input:-$default}"
    else
        read "input?$message: "
        echo "$input"
    fi
}

prompt_password() {
    local message="$1"
    local pass1
    local pass2

    while true; do
        read -s "pass1?$message: "
        echo "" >&2  # Output to stderr, not stdout (avoid capture by $())
        read -s "pass2?确认密码: "
        echo "" >&2  # Output to stderr, not stdout

        if [ "$pass1" = "$pass2" ] && [ -n "$pass1" ]; then
            printf '%s' "$pass1"  # Use printf without newline
            return 0
        fi

        print_error "密码不匹配或为空，请重试。"
    done
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

validate_url() {
    local url="$1"
    if [[ ! "$url" =~ ^https?:// ]]; then
        print_error "URL 格式无效，必须以 http:// 或 https:// 开头"
        return 1
    fi
    return 0
}

validate_port() {
    local port="$1"
    if [[ ! "$port" =~ ^[0-9]+$ ]] || [ "$port" -lt 1 ] || [ "$port" -gt 65535 ]; then
        print_error "端口号无效，必须在 1 到 65535 之间"
        return 1
    fi
    return 0
}

check_command() {
    local cmd="$1"
    if ! command -v "$cmd" &> /dev/null; then
        print_error "$cmd 未安装"
        return 1
    fi
    print_success "$cmd 已安装"
    return 0
}

configmap_file_value() {
    local key="$1"
    awk -F'"' -v lookup="$key" '$1 ~ "^[[:space:]]*" lookup ":" { print $2; exit }' "$K8S_DIR/configmap.yaml"
}

validate_static_configmap() {
    if [ -n "$SKIP_VALIDATION" ]; then
        print_warning "跳过静态 ConfigMap 占位符检查 (--skip-validation)"
        return 0
    fi

    print_info "检查 deploy/k8s/configmap.yaml 是否包含 example.com 占位符..."

    local has_placeholder=""
    local fields=(JWT_ISSUER WEBAUTHN_RP_ID CORS_ALLOWED_ORIGINS APP_BASE_URL AUTH9_CORE_PUBLIC_URL AUTH9_PORTAL_URL)

    for field in "${fields[@]}"; do
        local value
        value="$(configmap_file_value "$field")"
        if [[ "$value" == *"example.com"* ]]; then
            print_error "$field 仍是示例域名: $value"
            has_placeholder="true"
        fi
    done

    if [ -n "$has_placeholder" ]; then
        echo ""
        print_error "deploy/k8s/configmap.yaml 仍包含 example.com 占位符，中止非交互部署"
        print_info "请先修改 deploy/k8s/configmap.yaml 中标记为 'REQUIRED: replace before deploy' 的字段"
        print_info "或改用交互模式 ./deploy/deploy.sh 由脚本直接生成 ConfigMap"
        print_info "如需强制跳过，请追加 --skip-validation"
        exit 1
    fi

    print_success "静态 ConfigMap 无 example.com 占位符"
}

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

    print_warning "检测到旧版 Keycloak 资源仍存在；新清单不会自动清理这些遗留对象："
    echo "$resources" | sed 's/^/    /'
    print_info "确认 auth9-oidc 迁移稳定后，请手动删除这些旧资源，避免继续占用容量和误导运维排障"
}

generate_strong_admin_password() {
    # Ensure the seeded admin password always satisfies the default password policy:
    # uppercase + lowercase + digit + special + sufficient length.
    local suffix
    suffix=$(openssl rand -base64 48 | tr -dc 'A-Za-z0-9' | head -c 16)
    printf 'A9!a%s' "$suffix"
}

################################################################################
# Phase 2: Detection Logic
################################################################################

check_prerequisites() {
    local all_ok=true

    check_command "kubectl" || all_ok=false
    check_command "openssl" || all_ok=false
    check_command "base64" || all_ok=false

    # Check cluster access
    if kubectl cluster-info &> /dev/null; then
        print_success "集群已连接"
    else
        print_error "无法连接到 Kubernetes 集群"
        all_ok=false
    fi

    if [ "$all_ok" = false ]; then
        exit 1
    fi
}

detect_existing_secrets() {
    local secret_name="$1"
    local namespace="$2"
    local array_name="$3"
    shift 3
    local keys=("$@")

    if ! kubectl get secret "$secret_name" -n "$namespace" &>/dev/null; then
        print_warning "$secret_name 未找到（将创建）"
        return 1
    fi

    local found_count=0
    for key in "${keys[@]}"; do
        local value=$(kubectl get secret "$secret_name" -n "$namespace" -o jsonpath="{.data.$key}" 2>/dev/null | base64 -d 2>/dev/null || echo "")
        if [ -n "$value" ]; then
            eval "${array_name}[$key]=\"\$value\""
            ((found_count++))
        fi
    done

    if [ $found_count -gt 0 ]; then
        print_info "$secret_name 已找到（${found_count}/${#keys[@]} 个密钥）"
        return 0
    else
        print_warning "$secret_name 存在但为空"
        return 1
    fi
}

detect_existing_configmap() {
    if ! kubectl get configmap auth9-config -n "$NAMESPACE" &>/dev/null; then
        print_warning "auth9-config ConfigMap 未找到（将创建）"
        return 1
    fi

    # Read JWT_ISSUER and URLs
    local jwt_issuer=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.JWT_ISSUER}' 2>/dev/null || echo "")
    local core_public_url=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.AUTH9_CORE_PUBLIC_URL}' 2>/dev/null || echo "")
    local portal_url=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.AUTH9_PORTAL_URL}' 2>/dev/null || echo "")
    local portal_client_id=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.AUTH9_PORTAL_CLIENT_ID}' 2>/dev/null || echo "")

    if [ -n "$jwt_issuer" ]; then
        CONFIGMAP_VALUES[JWT_ISSUER]="$jwt_issuer"
        [ -n "$core_public_url" ] && CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$core_public_url"
        [ -n "$portal_url" ] && CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$portal_url"
        [ -n "$portal_client_id" ] && CONFIGMAP_VALUES[AUTH9_PORTAL_CLIENT_ID]="$portal_client_id"
        print_info "auth9-config ConfigMap 已找到"
        return 0
    fi

    print_warning "auth9-config ConfigMap 存在但未找到 JWT_ISSUER"
    return 1
}

################################################################################
# Phase 3: Interactive Input Collection
################################################################################

collect_database_config() {
    print_info "数据库配置"

    # Check if DATABASE_URL already exists
    if [ -n "${AUTH9_SECRETS[DATABASE_URL]}" ]; then
        echo "  当前: ${AUTH9_SECRETS[DATABASE_URL]%%\?*}"  # Hide password in URL
        if confirm_action "  保留现有数据库配置？"; then
            return 0
        fi
    fi

    # Collect components
    local db_host=$(prompt_user "  数据库主机" "advanced-tidb-tidb.tidb-system")
    local db_port=$(prompt_user "  数据库端口" "4000")

    while ! validate_port "$db_port"; do
        db_port=$(prompt_user "  数据库端口" "4000")
    done

    local db_username=$(prompt_user "  数据库用户名" "root")
    local db_password=$(prompt_password "  数据库密码")
    local db_name=$(prompt_user "  数据库名" "auth9")

    # Assemble URL
    AUTH9_SECRETS[DATABASE_URL]="mysql://${db_username}:${db_password}@${db_host}:${db_port}/${db_name}"
    print_success "DATABASE_URL 已配置"
}

collect_redis_config() {
    print_info "Redis 配置"

    if [ -n "${AUTH9_SECRETS[REDIS_URL]}" ]; then
        echo "  当前: ${AUTH9_SECRETS[REDIS_URL]}"
        if confirm_action "  保留现有 Redis 配置？"; then
            return 0
        fi
    fi

    local redis_host=$(prompt_user "  Redis 主机" "redis")
    local redis_port=$(prompt_user "  Redis 端口" "6379")

    while ! validate_port "$redis_port"; do
        redis_port=$(prompt_user "  Redis 端口" "6379")
    done

    AUTH9_SECRETS[REDIS_URL]="redis://${redis_host}:${redis_port}"
    print_success "REDIS_URL 已配置"
}

collect_jwt_issuer() {
    # JWT_ISSUER must always equal AUTH9_CORE_PUBLIC_URL (used for OAuth callback + token iss claim).
    # Auto-derive from AUTH9_CORE_PUBLIC_URL instead of asking separately.
    local core_url="${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}"
    CONFIGMAP_VALUES[JWT_ISSUER]="$core_url"
    print_info "JWT Issuer 自动设置为 Core 公网 URL: $core_url"
}

collect_core_public_url() {
    print_info "Auth9 Core 公网 URL 配置"

    local current="${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}"
    echo "  当前: $current"
    echo "  这是用于浏览器端 OAuth 重定向的 cloudflared 隧道 URL"

    if confirm_action "  修改 Auth9 Core 公网 URL？"; then
        local new_url
        while true; do
            new_url=$(prompt_user "  Auth9 Core 公网 URL（cloudflared 隧道）" "$current")
            if validate_url "$new_url"; then
                CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$new_url"
                break
            fi
        done
    else
        CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$current"
    fi

    print_success "Auth9 Core 公网 URL 已配置"
}

collect_portal_url() {
    print_info "Auth9 Portal URL 配置"

    local current="${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.example.com}"
    echo "  当前: $current"
    echo "  这是 Portal 的 cloudflared 隧道 URL"

    if confirm_action "  修改 Auth9 Portal URL？"; then
        local new_url
        while true; do
            new_url=$(prompt_user "  Auth9 Portal URL（cloudflared 隧道）" "$current")
            if validate_url "$new_url"; then
                CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$new_url"
                break
            fi
        done
    else
        CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$current"
    fi

    # Auto-derive WEBAUTHN_RP_ID from Portal URL (registrable domain)
    # e.g. https://auth9.example.com → example.com
    local portal_host=$(echo "${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]}" | sed -E 's|https?://||' | sed 's|/.*||' | sed 's|:.*||')
    # Extract registrable domain (last two parts)
    local rp_id=$(echo "$portal_host" | awk -F. '{if(NF>=2) print $(NF-1)"."$NF; else print $0}')
    CONFIGMAP_VALUES[WEBAUTHN_RP_ID]="$rp_id"
    print_info "WEBAUTHN_RP_ID 自动设置为: $rp_id"

    print_success "Auth9 Portal URL 已配置"
}

collect_admin_email() {
    print_info "Auth9 管理员邮箱配置"

    if [ -n "${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}" ]; then
        echo "  当前: ${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}"
        if confirm_action "  保留现有管理员邮箱？"; then
            # Sync to PLATFORM_ADMIN_EMAILS even when keeping existing value
            CONFIGMAP_VALUES[PLATFORM_ADMIN_EMAILS]="${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}"
            AUTH9_ADMIN_USERNAME="${AUTH9_ADMIN_USERNAME:-admin}"
            return 0
        fi
    fi

    local email=$(prompt_user "  管理员邮箱" "admin@auth9.local")
    AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]="$email"
    CONFIGMAP_VALUES[PLATFORM_ADMIN_EMAILS]="$email"
    AUTH9_ADMIN_USERNAME="${AUTH9_ADMIN_USERNAME:-admin}"
    print_success "AUTH9_ADMIN_EMAIL 已配置"
}

collect_observability_preference() {
    print_info "可观测性资源部署"

    if [ "$OBSERVABILITY_MODE" != "auto" ]; then
        print_info "可观测性部署模式已通过命令行指定: $OBSERVABILITY_MODE"
        return 0
    fi

    if confirm_action "  部署可观测性资源（ServiceMonitor / PrometheusRule / Grafana dashboards）？"; then
        OBSERVABILITY_MODE="enabled"
    else
        OBSERVABILITY_MODE="disabled"
    fi

    print_success "可观测性部署模式: $OBSERVABILITY_MODE"
}

generate_secrets() {
    # AUTH9 admin password used by auth9-init seeder for the initial platform admin.
    # Keep existing value stable to avoid accidental credential rotation on re-deploy.
    if [ -z "${AUTH9_SECRETS[AUTH9_ADMIN_PASSWORD]}" ]; then
        AUTH9_SECRETS[AUTH9_ADMIN_PASSWORD]=$(generate_strong_admin_password)
        AUTH9_ADMIN_PASSWORD="${AUTH9_SECRETS[AUTH9_ADMIN_PASSWORD]}"
        echo ""
        print_warning "已生成 AUTH9_ADMIN_PASSWORD - 请立即安全保存："
        echo -e "${GREEN}${AUTH9_SECRETS[AUTH9_ADMIN_PASSWORD]}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        AUTH9_ADMIN_PASSWORD="${AUTH9_SECRETS[AUTH9_ADMIN_PASSWORD]}"
        print_info "AUTH9_ADMIN_PASSWORD 已存在（不会重新生成）"
    fi

    # JWT_SECRET
    if [ -z "${AUTH9_SECRETS[JWT_SECRET]}" ]; then
        AUTH9_SECRETS[JWT_SECRET]=$(openssl rand -hex 32)
        echo ""
        print_warning "已生成 JWT_SECRET - 请安全保存："
        echo -e "${GREEN}${AUTH9_SECRETS[JWT_SECRET]}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        print_info "JWT_SECRET 已存在（不会重新生成）"
    fi

    # SESSION_SECRET
    if [ -z "${AUTH9_SECRETS[SESSION_SECRET]}" ]; then
        AUTH9_SECRETS[SESSION_SECRET]=$(openssl rand -hex 32)
        echo ""
        print_warning "已生成 SESSION_SECRET - 请安全保存："
        echo -e "${GREEN}${AUTH9_SECRETS[SESSION_SECRET]}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        print_info "SESSION_SECRET 已存在（不会重新生成）"
    fi

    # GRPC_API_KEYS
    if [ -z "${AUTH9_SECRETS[GRPC_API_KEYS]}" ]; then
        AUTH9_SECRETS[GRPC_API_KEYS]=$(openssl rand -base64 32)
        echo ""
        print_warning "已生成 GRPC_API_KEYS - 请安全保存："
        echo -e "${GREEN}${AUTH9_SECRETS[GRPC_API_KEYS]}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        print_info "GRPC_API_KEYS 已存在（不会重新生成）"
    fi

    # PASSWORD_RESET_HMAC_KEY (for password reset token signing)
    if [ -z "${AUTH9_SECRETS[PASSWORD_RESET_HMAC_KEY]}" ]; then
        AUTH9_SECRETS[PASSWORD_RESET_HMAC_KEY]=$(openssl rand -hex 32)
        echo ""
        print_warning "已生成 PASSWORD_RESET_HMAC_KEY - 请安全保存："
        echo -e "${GREEN}${AUTH9_SECRETS[PASSWORD_RESET_HMAC_KEY]}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        print_info "PASSWORD_RESET_HMAC_KEY 已存在（不会重新生成）"
    fi

    # SETTINGS_ENCRYPTION_KEY (AES-256 for encrypting sensitive settings)
    if [ -z "${AUTH9_SECRETS[SETTINGS_ENCRYPTION_KEY]}" ]; then
        AUTH9_SECRETS[SETTINGS_ENCRYPTION_KEY]=$(openssl rand -base64 32)
        echo ""
        print_warning "已生成 SETTINGS_ENCRYPTION_KEY - 请安全保存："
        echo -e "${GREEN}${AUTH9_SECRETS[SETTINGS_ENCRYPTION_KEY]}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        print_info "SETTINGS_ENCRYPTION_KEY 已存在（不会重新生成）"
    fi

    # JWT RSA Key Pair (RS256)
    if [ -z "${AUTH9_SECRETS[JWT_PRIVATE_KEY]}" ] || [ -z "${AUTH9_SECRETS[JWT_PUBLIC_KEY]}" ]; then
        print_info "正在生成 JWT RSA 密钥对（RS256）..."
        local tmp_private=$(mktemp)
        local tmp_public=$(mktemp)
        openssl genrsa -out "$tmp_private" 2048 2>/dev/null
        openssl rsa -in "$tmp_private" -pubout -out "$tmp_public" 2>/dev/null
        AUTH9_SECRETS[JWT_PRIVATE_KEY]=$(cat "$tmp_private")
        AUTH9_SECRETS[JWT_PUBLIC_KEY]=$(cat "$tmp_public")
        rm -f "$tmp_private" "$tmp_public"
        echo ""
        print_warning "已生成 JWT RSA 密钥对"
        echo ""
        read "?按 Enter 继续..."
    else
        print_info "JWT RSA 密钥对已存在（不会重新生成）"
    fi
}

################################################################################
# Phase 4: Configuration Management
################################################################################

create_or_patch_secret() {
    local secret_name=$1
    local namespace=$2
    local array_name=$3

    # Get keys from the associative array using eval
    local keys=()
    eval 'keys=(${(k)'$array_name'})'
    local key_count=${#keys[@]}

    if kubectl get secret "$secret_name" -n "$namespace" &>/dev/null; then
        # Secret exists, use patch
        print_info "正在更新 $secret_name..."

        for key in "${keys[@]}"; do
            local value
            eval 'value="${'$array_name'[$key]}"'
            local value_b64=$(echo -n "$value" | base64 | tr -d '\n')
            local patch_add="[{\"op\":\"add\",\"path\":\"/data/$key\",\"value\":\"$value_b64\"}]"
            local patch_replace="[{\"op\":\"replace\",\"path\":\"/data/$key\",\"value\":\"$value_b64\"}]"

            # Try add first, if it fails try replace
            if ! kubectl patch secret "$secret_name" -n "$namespace" --type=json -p="$patch_add" 2>/dev/null; then
                kubectl patch secret "$secret_name" -n "$namespace" --type=json -p="$patch_replace" 2>/dev/null || {
                    print_error "更新 $secret_name 中的 $key 失败"
                    return 1
                }
            fi
        done

        print_success "$secret_name 已更新（${key_count} 个密钥）"
    else
        # Secret doesn't exist, create it
        print_info "正在创建 $secret_name..."

        local create_cmd="kubectl create secret generic $secret_name"
        for key in "${keys[@]}"; do
            local value
            eval 'value="${'$array_name'[$key]}"'
            # Escape single quotes in value
            local escaped_value="${value//\'/\'\\\'\'}"
            create_cmd+=" --from-literal=$key='${escaped_value}'"
        done
        create_cmd+=" -n $namespace"

        if eval "$create_cmd"; then
            print_success "$secret_name 已创建（${key_count} 个密钥）"
        else
            print_error "创建 $secret_name 失败"
            return 1
        fi
    fi
}

apply_configmap() {
    # JWT_ISSUER must be the Core API URL (used for OAuth callback)
    local jwt_issuer="${CONFIGMAP_VALUES[JWT_ISSUER]:-${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}}"
    local portal_url="${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.example.com}"
    local cors_allowed_origins="$portal_url"

    cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: auth9-config
  namespace: $NAMESPACE
data:
  RUST_LOG: "info"
  HTTP_HOST: "0.0.0.0"
  HTTP_PORT: "8080"
  GRPC_HOST: "0.0.0.0"
  GRPC_PORT: "50051"
  DATABASE_MAX_CONNECTIONS: "10"
  DATABASE_MIN_CONNECTIONS: "2"
  JWT_ISSUER: "$jwt_issuer"
  JWT_ACCESS_TOKEN_TTL_SECS: "3600"
  JWT_REFRESH_TOKEN_TTL_SECS: "604800"
  JWT_TENANT_ACCESS_ALLOWED_AUDIENCES: "${CONFIGMAP_VALUES[AUTH9_PORTAL_CLIENT_ID]:-auth9-portal}"
  PASSWORD_RESET_TOKEN_TTL_SECS: "3600"
  GRPC_AUTH_MODE: "api_key"
  GRPC_ENABLE_REFLECTION: "false"
  WEBAUTHN_RP_ID: "${CONFIGMAP_VALUES[WEBAUTHN_RP_ID]:-auth9.example.com}"
  CORS_ALLOWED_ORIGINS: "$cors_allowed_origins"
  CORS_ALLOW_CREDENTIALS: "true"
  ACTION_ALLOWED_DOMAINS: "gitski.work,c9r.io"
  HSTS_ENABLED: "true"
  HSTS_HTTPS_ONLY: "true"
  HSTS_TRUST_X_FORWARDED_PROTO: "true"
  PLATFORM_ADMIN_EMAILS: "${CONFIGMAP_VALUES[PLATFORM_ADMIN_EMAILS]:-admin@auth9.local}"
  APP_BASE_URL: "$portal_url"
  AUTH9_CORE_URL: "http://auth9-core:8080"
  AUTH9_CORE_PUBLIC_URL: "${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}"
  AUTH9_PORTAL_URL: "$portal_url"
  AUTH9_PORTAL_CLIENT_ID: "${CONFIGMAP_VALUES[AUTH9_PORTAL_CLIENT_ID]:-auth9-portal}"
  ENVIRONMENT: "production"
  NODE_ENV: "production"
  OTEL_METRICS_ENABLED: "true"
  OTEL_TRACING_ENABLED: "false"
  LOG_FORMAT: "json"
  OTEL_SERVICE_NAME: "auth9-core"
EOF

    if [ $? -eq 0 ]; then
        print_success "ConfigMap 已应用"
    else
        print_error "应用 ConfigMap 失败"
        return 1
    fi
}

################################################################################
# Phase 5: Main Interactive Setup Flow
################################################################################

run_interactive_setup() {
    print_header "Auth9 交互式配置"

    # Step 1/6: Check prerequisites
    print_progress "1/6" "检查前提条件"
    check_prerequisites

    # Step 2/6: Detect existing configuration
    print_progress "2/6" "检测现有配置"

    # Detect auth9-secrets
    detect_existing_secrets "auth9-secrets" "$NAMESPACE" AUTH9_SECRETS \
        "DATABASE_URL" "REDIS_URL" "JWT_SECRET" "JWT_PRIVATE_KEY" "JWT_PUBLIC_KEY" \
        "SESSION_SECRET" "SETTINGS_ENCRYPTION_KEY" "PASSWORD_RESET_HMAC_KEY" "AUTH9_ADMIN_PASSWORD" \
        "GRPC_API_KEYS" "AUTH9_ADMIN_EMAIL" || true

    # Detect ConfigMap
    detect_existing_configmap || true

    # Step 3/6: Collect missing configuration
    print_progress "3/6" "收集配置信息"
    collect_database_config
    collect_redis_config
    collect_core_public_url
    collect_jwt_issuer
    collect_portal_url
    collect_admin_email
    collect_observability_preference

    # Step 4/6: Generate secrets
    print_progress "4/6" "生成安全密钥"
    generate_secrets

    # Step 5/6: Apply configuration
    print_progress "5/6" "应用配置到集群"

    # Create namespace if it doesn't exist
    kubectl create namespace "$NAMESPACE" 2>/dev/null || true

    # Apply secrets
    create_or_patch_secret "auth9-secrets" "$NAMESPACE" AUTH9_SECRETS

    # Apply ConfigMap
    apply_configmap

    # Step 6/6: Confirm deployment
    print_progress "6/6" "准备部署"
    print_summary

    if ! confirm_action "继续部署？"; then
        print_info "配置已保存。再次运行 deploy.sh 以部署。"
        exit 0
    fi
}

print_summary() {
    echo ""
    echo -e "${BOLD}配置摘要:${NC}"
    echo "  数据库: ${AUTH9_SECRETS[DATABASE_URL]%%\?*}"  # Hide password
    echo "  Redis: ${AUTH9_SECRETS[REDIS_URL]}"
    echo "  JWT Issuer: ${CONFIGMAP_VALUES[JWT_ISSUER]:-${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}}"
    echo "  Core 公网 URL: ${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}"
    echo "  Portal URL: ${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.example.com}"
    echo "  可观测性资源: $OBSERVABILITY_MODE"
    echo ""
}

################################################################################
# Phase 6: Enhanced Deployment Flow
################################################################################

deploy_auth9() {
    print_header "Auth9 部署"

    warn_deprecated_keycloak_resources

    # Step 1: Create namespace and service account
    print_progress "1/7" "创建命名空间和服务账户"
    kubectl apply -f "$K8S_DIR/namespace.yaml" $DRY_RUN
    kubectl apply -f "$K8S_DIR/serviceaccount.yaml" $DRY_RUN

    # Step 2: ConfigMap already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "2/7" "应用 ConfigMap"
        validate_static_configmap
        kubectl apply -f "$K8S_DIR/configmap.yaml" $DRY_RUN
    else
        print_progress "2/7" "ConfigMap 已应用"
    fi

    # Step 3: Secrets already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "3/7" "检查密钥"
        check_secrets_non_interactive
    else
        print_progress "3/7" "密钥已应用"
    fi

    # Step 4: Deploy infrastructure (redis)
    print_progress "4/7" "部署基础设施"
    deploy_infrastructure

    # Step 5: Deploy auth9 applications
    print_progress "5/7" "部署 auth9 应用"
    deploy_auth9_apps

    # Step 6: Deploy observability resources (ServiceMonitor, PrometheusRule, Grafana dashboards)
    print_progress "6/7" "部署可观测性资源"
    deploy_observability

    # Step 7: Wait for auth9 apps to be ready
    if [ -z "$DRY_RUN" ]; then
        print_progress "7/7" "等待 auth9 应用就绪"
        wait_for_auth9_apps
    else
        print_progress "7/7" "跳过等待（预演模式）"
    fi

    print_deployment_complete
}

check_secrets_non_interactive() {
    if kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
        print_success "auth9-secrets 存在"
    else
        print_warning "auth9-secrets 未找到，请创建："
        echo "    kubectl create secret generic auth9-secrets \\"
        echo "      --from-literal=DATABASE_URL='...' \\"
        echo "      --from-literal=REDIS_URL='...' \\"
        echo "      --from-literal=JWT_SECRET='...' \\"
        echo "      --from-literal=JWT_PRIVATE_KEY='...' \\"
        echo "      --from-literal=JWT_PUBLIC_KEY='...' \\"
        echo "      --from-literal=SESSION_SECRET='...' \\"
        echo "      --from-literal=SETTINGS_ENCRYPTION_KEY='...' \\"
        echo "      --from-literal=PASSWORD_RESET_HMAC_KEY='...' \\"
        echo "      --from-literal=GRPC_API_KEYS='...' \\"
        echo "      -n $NAMESPACE"
        echo ""
        if [ -z "$DRY_RUN" ]; then
            print_warning "继续执行（缺少密钥可能导致部署失败）"
        fi
    fi
}

deploy_infrastructure() {
    print_info "正在部署 redis..."
    kubectl apply -f "$K8S_DIR/redis/" $DRY_RUN
    if [ -n "$DRY_RUN" ]; then
        print_success "基础设施预演完成"
    else
        print_success "基础设施已部署"
    fi
}

deploy_auth9_apps() {
    print_info "正在部署 auth9-core..."
    kubectl apply -f "$K8S_DIR/auth9-core/" $DRY_RUN

    print_info "正在部署 auth9-oidc..."
    kubectl apply -f "$K8S_DIR/auth9-oidc/" $DRY_RUN

    print_info "正在部署 auth9-portal..."
    kubectl apply -f "$K8S_DIR/auth9-portal/" $DRY_RUN

    if [ -n "$DRY_RUN" ]; then
        print_success "Auth9 应用预演完成"
    else
        print_success "Auth9 应用已部署"
    fi
}

deploy_observability() {
    local obs_dir="$K8S_DIR/observability"
    if [ ! -d "$obs_dir" ]; then
        print_info "可观测性目录不存在，跳过"
        return
    fi

    if [ "$OBSERVABILITY_MODE" = "disabled" ]; then
        print_info "已配置跳过可观测性资源部署"
        return
    fi

    local has_service_monitor_crd="false"
    local has_prometheus_rule_crd="false"
    if kubectl get crd servicemonitors.monitoring.coreos.com &>/dev/null; then
        has_service_monitor_crd="true"
    fi
    if kubectl get crd prometheusrules.monitoring.coreos.com &>/dev/null; then
        has_prometheus_rule_crd="true"
    fi

    if [ "$has_service_monitor_crd" != "true" ] || [ "$has_prometheus_rule_crd" != "true" ]; then
        print_warning "集群未安装 Prometheus Operator CRD，跳过 ServiceMonitor / PrometheusRule 部署"
        print_info "如需启用，请先安装 servicemonitors.monitoring.coreos.com 和 prometheusrules.monitoring.coreos.com"
        if [ "$OBSERVABILITY_MODE" = "enabled" ]; then
            print_warning "已显式请求部署可观测性资源，但当前集群不支持这些 CRD"
        fi
        return
    fi

    print_info "正在部署可观测性资源 (ServiceMonitor, PrometheusRule, Grafana dashboards)..."
    kubectl apply -f "$obs_dir/" $DRY_RUN
    if [ -n "$DRY_RUN" ]; then
        print_success "可观测性资源预演完成"
    else
        print_success "可观测性资源已部署"
    fi
}

wait_for_auth9_apps() {
    print_info "等待 auth9-core..."
    kubectl rollout status deployment/auth9-core -n "$NAMESPACE" --timeout=300s || true

    print_info "等待 auth9-oidc..."
    kubectl rollout status deployment/auth9-oidc -n "$NAMESPACE" --timeout=300s || true

    print_info "等待 auth9-portal..."
    kubectl rollout status deployment/auth9-portal -n "$NAMESPACE" --timeout=300s || true

    print_info "等待 redis..."
    kubectl rollout status deployment/redis -n "$NAMESPACE" --timeout=300s || true
}

print_deployment_complete() {
    echo ""
    print_header "部署完成！"

    if [ -z "$DRY_RUN" ]; then
        echo -e "${YELLOW}当前 Pod 状态:${NC}"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/part-of=auth9
        echo ""
        echo -e "${YELLOW}服务:${NC}"
        kubectl get svc -n "$NAMESPACE"
        echo ""
        echo -e "${YELLOW}注意:${NC} 使用 cloudflared 暴露服务。详见 wiki/安装部署.md"
        echo -e "${YELLOW}建议:${NC} 部署完成后执行 ./scripts/validate-k8s-deploy.sh --namespace $NAMESPACE"
        echo ""
        echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${CYAN}║  Cloudflared 配置                                               ║${NC}"
        echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo -e "${BOLD}服务 URL:${NC}"
        echo ""
        local portal_url="${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.example.com}"
        local core_url="${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}"
        echo -e "  ${GREEN}auth9-portal（管理后台）:${NC}"
        echo -e "    公网 URL:     ${YELLOW}${portal_url}${NC}"
        echo -e "    内部地址:     auth9-portal.$NAMESPACE.svc.cluster.local:3000"
        echo ""
        echo -e "  ${GREEN}auth9-core（后端 API）:${NC}"
        echo -e "    公网 URL:     ${YELLOW}${core_url}${NC}"
        echo -e "    内部地址:     auth9-core.$NAMESPACE.svc.cluster.local:8080"
        echo ""
        echo -e "  ${GREEN}auth9-oidc（OIDC 身份引擎）:${NC}"
        echo -e "    内部地址:     auth9-oidc.$NAMESPACE.svc.cluster.local:8090"
        echo ""

        # Display admin credentials if extracted
        if [ -n "$AUTH9_ADMIN_PASSWORD" ]; then
            echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
            echo -e "${CYAN}║  Auth9 管理员凭据                                               ║${NC}"
            echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
            echo ""
            echo -e "  ${RED}${BOLD}重要: 请安全保存这些凭据！${NC}"
            echo ""
            echo -e "  ${GREEN}用户名:${NC}  ${YELLOW}${AUTH9_ADMIN_USERNAME}${NC}"
            if [ -n "${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}" ]; then
                echo -e "  ${GREEN}邮箱:${NC}    ${YELLOW}${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}${NC}"
            fi
            echo -e "  ${GREEN}密码:${NC}    ${YELLOW}${AUTH9_ADMIN_PASSWORD}${NC}"
            echo ""
            echo -e "  ${DIM}登录地址: ${portal_url}${NC}"
            echo ""
        fi
    fi
}

################################################################################
# Main Entry Point
################################################################################

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --interactive)
                INTERACTIVE="true"
                shift
                ;;
            --non-interactive)
                INTERACTIVE="false"
                shift
                ;;
            --dry-run)
                DRY_RUN="--dry-run=client"
                shift
                ;;
            --namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            --config-file)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --with-observability)
                OBSERVABILITY_MODE="enabled"
                shift
                ;;
            --without-observability)
                OBSERVABILITY_MODE="disabled"
                shift
                ;;
            --skip-validation)
                SKIP_VALIDATION="true"
                shift
                ;;
            *)
                echo -e "${RED}未知选项: $1${NC}"
                echo ""
                echo "用法: $0 [选项]"
                echo ""
                echo "选项:"
                echo "  --interactive       启用交互模式（默认）"
                echo "  --non-interactive   禁用交互模式"
                echo "  --dry-run           仅打印将要执行的操作，不实际执行"
                echo "  --namespace NS      使用其他命名空间（默认: auth9）"
                echo "  --config-file FILE  从文件加载配置"
                echo "  --with-observability    强制部署可观测性资源"
                echo "  --without-observability 跳过可观测性资源部署"
                echo "  --skip-validation       在非交互模式下跳过 ConfigMap 占位符检查"
                exit 1
                ;;
        esac
    done
}

main() {
    parse_arguments "$@"

    # Show mode
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║         Auth9 部署脚本                      ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}命名空间:${NC} $NAMESPACE"
    echo -e "${YELLOW}K8s 配置文件:${NC} $K8S_DIR"
    echo -e "${YELLOW}模式:${NC} $([ "$INTERACTIVE" = "true" ] && echo "交互式" || echo "非交互式")"
    if [ -n "$DRY_RUN" ]; then
        echo -e "${YELLOW}预演模式:${NC} 是"
    fi
    echo ""

    # Run interactive setup if enabled
    if [ "$INTERACTIVE" = "true" ] && [ -z "$DRY_RUN" ]; then
        run_interactive_setup
    fi

    # Deploy Auth9
    deploy_auth9
}

main "$@"
