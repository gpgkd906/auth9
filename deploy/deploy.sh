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
#   --skip-init         跳过 auth9-init 作业（已初始化时使用）
#   --namespace NS      使用其他命名空间（默认: auth9）
#   --config-file FILE  从文件加载配置（JSON 或 env 格式）
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
SKIP_INIT=""
INTERACTIVE="true"
CONFIG_FILE=""
NEEDS_INIT_JOB="false"

# Associative arrays for configuration
declare -A AUTH9_SECRETS
declare -A KEYCLOAK_SECRETS
declare -A CONFIGMAP_VALUES

# Admin credentials (extracted from init job)
AUTH9_ADMIN_USERNAME=""
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
    local keycloak_public_url=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.KEYCLOAK_PUBLIC_URL}' 2>/dev/null || echo "")

    if [ -n "$jwt_issuer" ]; then
        CONFIGMAP_VALUES[JWT_ISSUER]="$jwt_issuer"
        [ -n "$core_public_url" ] && CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$core_public_url"
        [ -n "$portal_url" ] && CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$portal_url"
        [ -n "$keycloak_public_url" ] && CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]="$keycloak_public_url"
        print_info "auth9-config ConfigMap 已找到"
        return 0
    fi

    print_warning "auth9-config ConfigMap 存在但未找到 JWT_ISSUER"
    return 1
}

should_run_init_job() {
    # If KEYCLOAK_ADMIN_CLIENT_SECRET already exists and is not empty, skip init
    if [ -n "${AUTH9_SECRETS[KEYCLOAK_ADMIN_CLIENT_SECRET]}" ]; then
        NEEDS_INIT_JOB="false"
        print_info "管理员客户端密钥已存在，可能不需要运行初始化作业"
    else
        NEEDS_INIT_JOB="true"
        print_info "管理员客户端密钥缺失，需要运行初始化作业"
    fi
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

collect_keycloak_public_url() {
    print_info "Keycloak 公网 URL 配置"

    local current="${CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]:-https://idp.auth9.example.com}"
    echo "  当前: $current"
    echo "  这是 Keycloak 的 cloudflared 隧道 URL（用于浏览器登录）"

    if confirm_action "  修改 Keycloak 公网 URL？"; then
        local new_url
        while true; do
            new_url=$(prompt_user "  Keycloak 公网 URL（cloudflared 隧道）" "$current")
            if validate_url "$new_url"; then
                CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]="$new_url"
                break
            fi
        done
    else
        CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]="$current"
    fi

    print_success "Keycloak 公网 URL 已配置"
}

collect_keycloak_config() {
    print_info "Keycloak 配置"

    # KEYCLOAK_URL (internal)
    if [ -z "${AUTH9_SECRETS[KEYCLOAK_URL]}" ]; then
        AUTH9_SECRETS[KEYCLOAK_URL]="http://keycloak:8080"
    fi

    # KEYCLOAK_ADMIN (default value)
    if [ -z "${AUTH9_SECRETS[KEYCLOAK_ADMIN]}" ]; then
        AUTH9_SECRETS[KEYCLOAK_ADMIN]="admin"
    fi
    KEYCLOAK_SECRETS[KEYCLOAK_ADMIN]="${AUTH9_SECRETS[KEYCLOAK_ADMIN]}"

    # KEYCLOAK_ADMIN_PASSWORD (shared between both secrets)
    if [ -n "${AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]}" ]; then
        echo "  Keycloak 管理员密码: （已配置）"
        if confirm_action "  修改 Keycloak 管理员密码？"; then
            local keycloak_password=$(prompt_password "  新 Keycloak 管理员密码")
            AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
            KEYCLOAK_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
        else
            KEYCLOAK_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="${AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]}"
        fi
    else
        local keycloak_password=$(prompt_password "  Keycloak 管理员密码")
        AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
        KEYCLOAK_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
    fi

    print_success "Keycloak 管理员已配置"
}

collect_keycloak_db_password() {
    print_info "Keycloak 数据库配置"

    # KC_DB_USERNAME (default value)
    if [ -z "${KEYCLOAK_SECRETS[KC_DB_USERNAME]}" ]; then
        KEYCLOAK_SECRETS[KC_DB_USERNAME]="keycloak"
    fi

    # KC_DB_PASSWORD
    if [ -n "${KEYCLOAK_SECRETS[KC_DB_PASSWORD]}" ]; then
        echo "  Keycloak 数据库密码: （已配置）"
        if confirm_action "  修改 Keycloak 数据库密码？"; then
            KEYCLOAK_SECRETS[KC_DB_PASSWORD]=$(prompt_password "  新 Keycloak 数据库密码")
        fi
    else
        KEYCLOAK_SECRETS[KC_DB_PASSWORD]=$(prompt_password "  Keycloak 数据库密码")
    fi

    print_success "Keycloak 数据库已配置"
}

collect_jwt_issuer() {
    print_info "JWT Issuer 配置"
    echo "  注意: JWT_ISSUER 必须是 Core API URL（用于 OAuth 回调）"

    # Default to Core API URL, not portal URL
    local current="${CONFIGMAP_VALUES[JWT_ISSUER]:-${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}}"
    echo "  当前 JWT Issuer: $current"

    if confirm_action "  修改 JWT Issuer？"; then
        local new_issuer
        while true; do
            new_issuer=$(prompt_user "  JWT Issuer URL" "$current")
            if validate_url "$new_issuer"; then
                CONFIGMAP_VALUES[JWT_ISSUER]="$new_issuer"
                break
            fi
        done
    else
        CONFIGMAP_VALUES[JWT_ISSUER]="$current"
    fi

    print_success "JWT Issuer 已配置"
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

    print_success "Auth9 Portal URL 已配置"
}

collect_admin_email() {
    print_info "Auth9 管理员邮箱配置"

    if [ -n "${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}" ]; then
        echo "  当前: ${AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]}"
        if confirm_action "  保留现有管理员邮箱？"; then
            return 0
        fi
    fi

    local email=$(prompt_user "  管理员邮箱" "admin@auth9.local")
    AUTH9_SECRETS[AUTH9_ADMIN_EMAIL]="$email"
    print_success "AUTH9_ADMIN_EMAIL 已配置"
}

generate_secrets() {
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

    # KEYCLOAK_WEBHOOK_SECRET (shared between auth9-secrets and keycloak-secrets)
    if [ -z "${AUTH9_SECRETS[KEYCLOAK_WEBHOOK_SECRET]}" ]; then
        local webhook_secret=$(openssl rand -hex 32)
        AUTH9_SECRETS[KEYCLOAK_WEBHOOK_SECRET]="$webhook_secret"
        KEYCLOAK_SECRETS[KC_SPI_EVENTS_LISTENER_EXT_EVENT_HTTP_HMAC_SECRET]="$webhook_secret"
        echo ""
        print_warning "已生成 KEYCLOAK_WEBHOOK_SECRET - 请安全保存："
        echo -e "${GREEN}${webhook_secret}${NC}"
        echo ""
        read "?保存后按 Enter 继续..."
    else
        print_info "KEYCLOAK_WEBHOOK_SECRET 已存在（不会重新生成）"
        # Sync to keycloak-secrets if not already set
        if [ -z "${KEYCLOAK_SECRETS[KC_SPI_EVENTS_LISTENER_EXT_EVENT_HTTP_HMAC_SECRET]}" ]; then
            KEYCLOAK_SECRETS[KC_SPI_EVENTS_LISTENER_EXT_EVENT_HTTP_HMAC_SECRET]="${AUTH9_SECRETS[KEYCLOAK_WEBHOOK_SECRET]}"
        fi
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
  PASSWORD_RESET_TOKEN_TTL_SECS: "3600"
  KEYCLOAK_REALM: "auth9"
  KEYCLOAK_ADMIN_CLIENT_ID: "auth9-admin"
  KEYCLOAK_SSL_REQUIRED: "none"
  KEYCLOAK_PUBLIC_URL: "${CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]:-https://idp.auth9.example.com}"
  GRPC_AUTH_MODE: "api_key"
  GRPC_ENABLE_REFLECTION: "false"
  WEBAUTHN_RP_ID: "${CONFIGMAP_VALUES[WEBAUTHN_RP_ID]:-auth9.example.com}"
  CORS_ALLOWED_ORIGINS: "${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.example.com}"
  CORS_ALLOW_CREDENTIALS: "true"
  PLATFORM_ADMIN_EMAILS: "${CONFIGMAP_VALUES[PLATFORM_ADMIN_EMAILS]:-admin@auth9.local}"
  AUTH9_CORE_URL: "http://auth9-core:8080"
  AUTH9_CORE_PUBLIC_URL: "${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.example.com}"
  AUTH9_PORTAL_URL: "${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.example.com}"
  AUTH9_PORTAL_CLIENT_ID: "auth9-portal"
  NODE_ENV: "production"
  OTEL_METRICS_ENABLED: "true"
  OTEL_TRACING_ENABLED: "true"
  OTEL_EXPORTER_OTLP_ENDPOINT: "http://tempo.observability:4317"
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
        "SESSION_SECRET" "SETTINGS_ENCRYPTION_KEY" \
        "KEYCLOAK_URL" "KEYCLOAK_ADMIN" "KEYCLOAK_ADMIN_PASSWORD" "KEYCLOAK_ADMIN_CLIENT_SECRET" \
        "KEYCLOAK_WEBHOOK_SECRET" "GRPC_API_KEYS" "AUTH9_ADMIN_EMAIL" || true

    # Detect keycloak-secrets
    detect_existing_secrets "keycloak-secrets" "$NAMESPACE" KEYCLOAK_SECRETS \
        "KEYCLOAK_ADMIN" "KEYCLOAK_ADMIN_PASSWORD" "KC_DB_USERNAME" "KC_DB_PASSWORD" \
        "KC_SPI_EVENTS_LISTENER_EXT_EVENT_HTTP_HMAC_SECRET" || true

    # Detect ConfigMap
    detect_existing_configmap || true

    # Check if init job is needed
    should_run_init_job

    echo ""
    print_info "是否需要初始化作业: $([ "$NEEDS_INIT_JOB" = "true" ] && echo "是" || echo "否（客户端密钥已存在）")"

    # Step 3/6: Collect missing configuration
    print_progress "3/6" "收集配置信息"
    collect_database_config
    collect_redis_config
    collect_keycloak_config
    collect_keycloak_db_password
    collect_jwt_issuer
    collect_core_public_url
    collect_portal_url
    collect_keycloak_public_url
    collect_admin_email

    # Step 4/6: Generate secrets
    print_progress "4/6" "生成安全密钥"
    generate_secrets

    # Step 5/6: Apply configuration
    print_progress "5/6" "应用配置到集群"

    # Create namespace if it doesn't exist
    kubectl create namespace "$NAMESPACE" 2>/dev/null || true

    # Apply secrets
    create_or_patch_secret "auth9-secrets" "$NAMESPACE" AUTH9_SECRETS
    create_or_patch_secret "keycloak-secrets" "$NAMESPACE" KEYCLOAK_SECRETS

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
    echo "  Keycloak 公网 URL: ${CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]:-https://idp.auth9.example.com}"
    echo "  初始化作业: $([ "$NEEDS_INIT_JOB" = "true" ] && echo "将运行" || echo "将跳过（客户端密钥已存在）")"
    echo ""
}

################################################################################
# Phase 6: Enhanced Deployment Flow
################################################################################

deploy_auth9() {
    print_header "Auth9 部署"

    # Step 1: Create namespace and service account
    print_progress "1/11" "创建命名空间和服务账户"
    kubectl apply -f "$K8S_DIR/namespace.yaml" $DRY_RUN
    kubectl apply -f "$K8S_DIR/serviceaccount.yaml" $DRY_RUN
    kubectl apply -f "$K8S_DIR/ghcr-secret.yaml" $DRY_RUN

    # Step 2: ConfigMap already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "2/11" "应用 ConfigMap"
        kubectl apply -f "$K8S_DIR/configmap.yaml" $DRY_RUN
    else
        print_progress "2/11" "ConfigMap 已应用"
    fi

    # Step 3: Secrets already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "3/11" "检查密钥"
        check_secrets_non_interactive
    else
        print_progress "3/11" "密钥已应用"
    fi

    # Step 4: Deploy infrastructure (keycloak, redis, postgres)
    print_progress "4/11" "部署基础设施"
    deploy_infrastructure

    # Step 5-6: Wait for dependencies
    print_progress "5/11" "等待 keycloak-postgres 就绪"
    wait_for_postgres

    print_progress "6/11" "等待 keycloak 就绪"
    wait_for_keycloak

    # Step 7-8: Init job (conditional execution) - runs AFTER keycloak is ready
    if [ "$NEEDS_INIT_JOB" = "true" ] && [ -z "$SKIP_INIT" ]; then
        print_progress "7/11" "运行 auth9-init 初始化作业"
        run_init_job

        print_progress "8/11" "提取 Keycloak 管理员客户端密钥"
        extract_client_secret
    else
        print_progress "7/11" "跳过 auth9-init 初始化作业"
        print_progress "8/11" "跳过密钥提取"
    fi

    # Step 9: Deploy auth9 applications
    print_progress "9/11" "部署 auth9 应用"
    deploy_auth9_apps

    # Step 10: Deploy observability resources (ServiceMonitor, PrometheusRule, Grafana dashboards)
    print_progress "10/11" "部署可观测性资源"
    deploy_observability

    # Step 11: Wait for auth9 apps to be ready
    if [ -z "$DRY_RUN" ]; then
        print_progress "11/11" "等待 auth9 应用就绪"
        wait_for_auth9_apps
    else
        print_progress "11/11" "跳过等待（预演模式）"
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
        echo "      --from-literal=KEYCLOAK_URL='...' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN='admin' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN_PASSWORD='...' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN_CLIENT_SECRET='<将自动生成>' \\"
        echo "      --from-literal=SESSION_SECRET='...' \\"
        echo "      --from-literal=SETTINGS_ENCRYPTION_KEY='...' \\"
        echo "      --from-literal=KEYCLOAK_WEBHOOK_SECRET='...' \\"
        echo "      --from-literal=GRPC_API_KEYS='...' \\"
        echo "      -n $NAMESPACE"
        echo ""
        if [ -z "$DRY_RUN" ]; then
            print_warning "继续执行（缺少密钥可能导致部署失败）"
        fi
    fi

    if kubectl get secret keycloak-secrets -n "$NAMESPACE" &> /dev/null; then
        print_success "keycloak-secrets 存在"
    else
        print_warning "keycloak-secrets 未找到，请创建："
        echo "    kubectl create secret generic keycloak-secrets \\"
        echo "      --from-literal=KEYCLOAK_ADMIN='admin' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN_PASSWORD='...' \\"
        echo "      --from-literal=KC_DB_USERNAME='keycloak' \\"
        echo "      --from-literal=KC_DB_PASSWORD='...' \\"
        echo "      --from-literal=KC_SPI_EVENTS_LISTENER_EXT_EVENT_HTTP_HMAC_SECRET='...' \\"
        echo "      -n $NAMESPACE"
        echo ""
        if [ -z "$DRY_RUN" ]; then
            print_warning "继续执行（缺少密钥可能导致部署失败）"
        fi
    fi
}

run_init_job() {
    if [ -z "$DRY_RUN" ]; then
        # Check if required secrets exist
        if ! kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
            print_error "auth9-secrets 未找到。初始化作业需要："
            echo "    - KEYCLOAK_ADMIN"
            echo "    - KEYCLOAK_ADMIN_PASSWORD"
            echo "    - DATABASE_URL"
            echo "    - REDIS_URL"
            echo "  请先创建密钥，然后重新运行此脚本。"
            exit 1
        fi

        # Delete old job if exists
        if kubectl get job auth9-init -n "$NAMESPACE" &> /dev/null; then
            print_info "正在删除现有的 auth9-init 作业..."
            kubectl delete job auth9-init -n "$NAMESPACE" --ignore-not-found=true
            sleep 2
        fi

        # Apply init job
        print_info "正在创建 auth9-init 作业..."
        kubectl apply -f "$K8S_DIR/auth9-core/init-job.yaml"

        # Wait for job to complete
        print_info "等待初始化作业完成（超时: 300秒）..."
        if kubectl wait --for=condition=complete job/auth9-init -n "$NAMESPACE" --timeout=300s 2>/dev/null; then
            print_success "初始化作业已成功完成"
        else
            print_error "初始化作业失败或超时"
            echo ""
            echo "  最近的日志:"
            kubectl logs job/auth9-init -n "$NAMESPACE" --tail=20 2>/dev/null || true
            echo ""
            echo "  完整日志: kubectl logs job/auth9-init -n $NAMESPACE"
            exit 1
        fi
    else
        print_info "跳过初始化作业（预演模式）"
    fi
}

extract_client_secret() {
    if [ -z "$DRY_RUN" ]; then
        # Get the secret from init job logs
        print_info "正在读取 auth9-init 作业日志..."
        local init_logs=$(kubectl logs job/auth9-init -n "$NAMESPACE" 2>/dev/null || echo "")

        # Extract admin credentials if present
        if echo "$init_logs" | grep -q "AUTH9_ADMIN_USERNAME="; then
            AUTH9_ADMIN_USERNAME=$(echo "$init_logs" | grep "AUTH9_ADMIN_USERNAME=" | sed 's/.*AUTH9_ADMIN_USERNAME=//' | head -1)
            AUTH9_ADMIN_PASSWORD=$(echo "$init_logs" | grep "AUTH9_ADMIN_PASSWORD=" | sed 's/.*AUTH9_ADMIN_PASSWORD=//' | head -1)
            if [ -n "$AUTH9_ADMIN_PASSWORD" ]; then
                print_success "已提取管理员凭据"
            fi
        fi

        if echo "$init_logs" | grep -q "KEYCLOAK_ADMIN_CLIENT_SECRET"; then
            local client_secret=$(echo "$init_logs" | grep "KEYCLOAK_ADMIN_CLIENT_SECRET=" | sed 's/.*KEYCLOAK_ADMIN_CLIENT_SECRET=//' | head -1)

            if [ -n "$client_secret" ]; then
                print_success "已提取客户端密钥: ${client_secret:0:8}..."
                echo ""
                echo -e "  ${BLUE}KEYCLOAK_ADMIN_CLIENT_SECRET:${NC}"
                echo "  $client_secret"
                echo ""

                # Update auth9-secrets with the new client secret
                if kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
                    print_info "正在使用新的 KEYCLOAK_ADMIN_CLIENT_SECRET 更新 auth9-secrets..."
                    local client_secret_b64=$(echo -n "$client_secret" | base64 | tr -d '\n')

                    if kubectl patch secret auth9-secrets -n "$NAMESPACE" \
                        --type='json' \
                        -p="[{\"op\": \"add\", \"path\": \"/data/KEYCLOAK_ADMIN_CLIENT_SECRET\", \"value\": \"$client_secret_b64\"}]" 2>/dev/null; then
                        print_success "密钥更新成功"
                    else
                        # Try replace if add fails
                        kubectl patch secret auth9-secrets -n "$NAMESPACE" \
                            --type='json' \
                            -p="[{\"op\": \"replace\", \"path\": \"/data/KEYCLOAK_ADMIN_CLIENT_SECRET\", \"value\": \"$client_secret_b64\"}]" 2>/dev/null || {
                            print_warning "更新密钥失败（可能已存在）"
                            echo "  手动更新命令:"
                            echo "    kubectl patch secret auth9-secrets -n $NAMESPACE --type='json' \\"
                            echo "      -p='[{\"op\": \"replace\", \"path\": \"/data/KEYCLOAK_ADMIN_CLIENT_SECRET\", \"value\": \"$client_secret_b64\"}]'"
                        }
                    fi
                else
                    print_warning "auth9-secrets 未找到，无法更新"
                    echo "  请手动添加: KEYCLOAK_ADMIN_CLIENT_SECRET=$client_secret"
                fi
            else
                print_warning "无法从日志中提取客户端密钥"
            fi
        else
            # Check if client already exists (idempotent operation)
            if echo "$init_logs" | grep -q "auth9-admin client already exists"; then
                print_info "auth9-admin 客户端已存在（跳过创建）"
                echo "  如需密钥，请从 Keycloak 管理控制台手动获取"
            else
                print_warning "在初始化日志中未找到客户端密钥"
                echo "  如果使用预设密钥或客户端已存在，这是预期行为"
            fi
        fi
    else
        print_info "跳过密钥提取（预演模式）"
    fi
}

deploy_infrastructure() {
    if [ -z "$DRY_RUN" ]; then
        print_info "正在部署 keycloak..."
        kubectl apply -f "$K8S_DIR/keycloak/" $DRY_RUN

        print_info "正在部署 redis..."
        kubectl apply -f "$K8S_DIR/redis/" $DRY_RUN

        print_success "基础设施已部署"
    else
        print_info "跳过基础设施部署（预演模式）"
    fi
}

deploy_auth9_apps() {
    if [ -z "$DRY_RUN" ]; then
        print_info "正在部署 auth9-core..."
        kubectl apply -f "$K8S_DIR/auth9-core/" $DRY_RUN

        print_info "正在部署 auth9-portal..."
        kubectl apply -f "$K8S_DIR/auth9-portal/" $DRY_RUN

        print_success "Auth9 应用已部署"
    else
        print_info "跳过 auth9 部署（预演模式）"
    fi
}

deploy_observability() {
    local obs_dir="$K8S_DIR/observability"
    if [ ! -d "$obs_dir" ]; then
        print_info "可观测性目录不存在，跳过"
        return
    fi

    if [ -z "$DRY_RUN" ]; then
        print_info "正在部署可观测性资源 (ServiceMonitor, PrometheusRule, Grafana dashboards)..."
        kubectl apply -f "$obs_dir/" $DRY_RUN
        print_success "可观测性资源已部署"
    else
        print_info "跳过可观测性部署（预演模式）"
    fi
}

wait_for_keycloak() {
    if [ -z "$DRY_RUN" ]; then
        print_info "等待 keycloak 部署..."
        kubectl rollout status deployment/keycloak -n "$NAMESPACE" --timeout=300s || true

        # Wait for all keycloak pods to be ready (using kubectl wait)
        print_info "等待 keycloak Pod 就绪..."
        if kubectl wait --for=condition=Ready pod -l app.kubernetes.io/name=keycloak -n "$NAMESPACE" --timeout=150s 2>/dev/null; then
            print_success "Keycloak 已就绪"
            return 0
        else
            print_warning "Keycloak 就绪检查超时，继续执行..."
        fi
    fi
}

wait_for_auth9_apps() {
    print_info "等待 auth9-core..."
    kubectl rollout status deployment/auth9-core -n "$NAMESPACE" --timeout=300s || true

    print_info "等待 auth9-portal..."
    kubectl rollout status deployment/auth9-portal -n "$NAMESPACE" --timeout=300s || true

    print_info "等待 redis..."
    kubectl rollout status deployment/redis -n "$NAMESPACE" --timeout=300s || true
}

wait_for_postgres() {
    if [ -z "$DRY_RUN" ]; then
        kubectl rollout status statefulset/keycloak-postgres -n "$NAMESPACE" --timeout=300s || true
    fi
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
        local keycloak_url="${CONFIGMAP_VALUES[KEYCLOAK_PUBLIC_URL]:-https://idp.auth9.example.com}"
        echo -e "  ${GREEN}keycloak（OIDC 提供者）:${NC}"
        echo -e "    公网 URL:     ${YELLOW}${keycloak_url}${NC}"
        echo -e "    内部地址:     keycloak.$NAMESPACE.svc.cluster.local:8080"
        echo -e "    ${DIM}（浏览器会重定向到 Keycloak 进行登录）${NC}"
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
            --skip-init)
                SKIP_INIT="true"
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
            *)
                echo -e "${RED}未知选项: $1${NC}"
                echo ""
                echo "用法: $0 [选项]"
                echo ""
                echo "选项:"
                echo "  --interactive       启用交互模式（默认）"
                echo "  --non-interactive   禁用交互模式"
                echo "  --dry-run           仅打印将要执行的操作，不实际执行"
                echo "  --skip-init         跳过 auth9-init 初始化作业"
                echo "  --namespace NS      使用其他命名空间（默认: auth9）"
                echo "  --config-file FILE  从文件加载配置"
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
