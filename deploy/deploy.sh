#!/usr/bin/env zsh
# Auth9 Interactive Deployment Script
#
# This script deploys Auth9 to a Kubernetes cluster with interactive configuration setup.
#
# Usage:
#   ./deploy.sh [options]
#
# Options:
#   --interactive       Enable interactive mode (default)
#   --non-interactive   Disable interactive mode, use original behavior
#   --dry-run           Print what would be applied without executing
#   --skip-init         Skip the auth9-init job (use if already initialized)
#   --namespace NS      Use a different namespace (default: auth9)
#   --config-file FILE  Load configuration from file (JSON or env format)
#
# Prerequisites:
#   - kubectl configured with cluster access
#   - openssl (for secret generation)
#   - base64 (for secret encoding)

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
trap 'print_error "Setup interrupted"; exit 130' INT TERM

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
        read -s "pass2?Confirm password: "
        echo "" >&2  # Output to stderr, not stdout

        if [ "$pass1" = "$pass2" ] && [ -n "$pass1" ]; then
            printf '%s' "$pass1"  # Use printf without newline
            return 0
        fi

        print_error "Passwords don't match or empty. Please try again."
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
            * ) echo "Please answer yes or no." ;;
        esac
    done
}

validate_url() {
    local url="$1"
    if [[ ! "$url" =~ ^https?:// ]]; then
        print_error "Invalid URL format. Must start with http:// or https://"
        return 1
    fi
    return 0
}

validate_port() {
    local port="$1"
    if [[ ! "$port" =~ ^[0-9]+$ ]] || [ "$port" -lt 1 ] || [ "$port" -gt 65535 ]; then
        print_error "Invalid port number. Must be between 1 and 65535."
        return 1
    fi
    return 0
}

check_command() {
    local cmd="$1"
    if ! command -v "$cmd" &> /dev/null; then
        print_error "$cmd is not installed"
        return 1
    fi
    print_success "$cmd installed"
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
        print_success "Cluster connected"
    else
        print_error "Cannot connect to Kubernetes cluster"
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
        print_warning "$secret_name not found (will create)"
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
        print_info "$secret_name found ($found_count/${#keys[@]} keys)"
        return 0
    else
        print_warning "$secret_name exists but is empty"
        return 1
    fi
}

detect_existing_configmap() {
    if ! kubectl get configmap auth9-config -n "$NAMESPACE" &>/dev/null; then
        print_warning "auth9-config ConfigMap not found (will create)"
        return 1
    fi

    # Read JWT_ISSUER and URLs
    local jwt_issuer=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.JWT_ISSUER}' 2>/dev/null || echo "")
    local core_public_url=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.AUTH9_CORE_PUBLIC_URL}' 2>/dev/null || echo "")
    local portal_url=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.AUTH9_PORTAL_URL}' 2>/dev/null || echo "")

    if [ -n "$jwt_issuer" ]; then
        CONFIGMAP_VALUES[JWT_ISSUER]="$jwt_issuer"
        [ -n "$core_public_url" ] && CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$core_public_url"
        [ -n "$portal_url" ] && CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$portal_url"
        print_info "auth9-config ConfigMap found"
        return 0
    fi

    print_warning "auth9-config ConfigMap exists but JWT_ISSUER not found"
    return 1
}

should_run_init_job() {
    # If KEYCLOAK_ADMIN_CLIENT_SECRET already exists and is not empty, skip init
    if [ -n "${AUTH9_SECRETS[KEYCLOAK_ADMIN_CLIENT_SECRET]}" ]; then
        NEEDS_INIT_JOB="false"
        print_info "Admin client secret exists, init job may not be needed"
    else
        NEEDS_INIT_JOB="true"
        print_info "Admin client secret missing, init job will be required"
    fi
}

################################################################################
# Phase 3: Interactive Input Collection
################################################################################

collect_database_config() {
    print_info "Database Configuration"

    # Check if DATABASE_URL already exists
    if [ -n "${AUTH9_SECRETS[DATABASE_URL]}" ]; then
        echo "  Current: ${AUTH9_SECRETS[DATABASE_URL]%%\?*}"  # Hide password in URL
        if confirm_action "  Keep existing database config?"; then
            return 0
        fi
    fi

    # Collect components
    local db_host=$(prompt_user "  Database host" "advanced-tidb-tidb.tidb-system")
    local db_port=$(prompt_user "  Database port" "4000")

    while ! validate_port "$db_port"; do
        db_port=$(prompt_user "  Database port" "4000")
    done

    local db_username=$(prompt_user "  Database username" "root")
    local db_password=$(prompt_password "  Database password")
    local db_name=$(prompt_user "  Database name" "auth9")

    # Assemble URL
    AUTH9_SECRETS[DATABASE_URL]="mysql://${db_username}:${db_password}@${db_host}:${db_port}/${db_name}"
    print_success "DATABASE_URL configured"
}

collect_redis_config() {
    print_info "Redis Configuration"

    if [ -n "${AUTH9_SECRETS[REDIS_URL]}" ]; then
        echo "  Current: ${AUTH9_SECRETS[REDIS_URL]}"
        if confirm_action "  Keep existing Redis config?"; then
            return 0
        fi
    fi

    local redis_host=$(prompt_user "  Redis host" "redis")
    local redis_port=$(prompt_user "  Redis port" "6379")

    while ! validate_port "$redis_port"; do
        redis_port=$(prompt_user "  Redis port" "6379")
    done

    AUTH9_SECRETS[REDIS_URL]="redis://${redis_host}:${redis_port}"
    print_success "REDIS_URL configured"
}

collect_keycloak_config() {
    print_info "Keycloak Configuration"

    # KEYCLOAK_URL
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
        echo "  Keycloak admin password: (already configured)"
        if confirm_action "  Change Keycloak admin password?"; then
            local keycloak_password=$(prompt_password "  New Keycloak admin password")
            AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
            KEYCLOAK_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
        else
            KEYCLOAK_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="${AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]}"
        fi
    else
        local keycloak_password=$(prompt_password "  Keycloak admin password")
        AUTH9_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
        KEYCLOAK_SECRETS[KEYCLOAK_ADMIN_PASSWORD]="$keycloak_password"
    fi

    print_success "Keycloak admin configured"
}

collect_keycloak_db_password() {
    print_info "Keycloak Database Configuration"

    # KC_DB_USERNAME (default value)
    if [ -z "${KEYCLOAK_SECRETS[KC_DB_USERNAME]}" ]; then
        KEYCLOAK_SECRETS[KC_DB_USERNAME]="keycloak"
    fi

    # KC_DB_PASSWORD
    if [ -n "${KEYCLOAK_SECRETS[KC_DB_PASSWORD]}" ]; then
        echo "  Keycloak DB password: (already configured)"
        if confirm_action "  Change Keycloak DB password?"; then
            KEYCLOAK_SECRETS[KC_DB_PASSWORD]=$(prompt_password "  New Keycloak DB password")
        fi
    else
        KEYCLOAK_SECRETS[KC_DB_PASSWORD]=$(prompt_password "  Keycloak DB password")
    fi

    print_success "Keycloak DB configured"
}

collect_jwt_issuer() {
    print_info "JWT Issuer Configuration"

    local current="${CONFIGMAP_VALUES[JWT_ISSUER]:-https://auth9.gitski.work}"
    echo "  Current JWT Issuer: $current"

    if confirm_action "  Change JWT Issuer?"; then
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

    print_success "JWT Issuer configured"
}

collect_core_public_url() {
    print_info "Auth9 Core Public URL Configuration"

    local current="${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.gitski.work}"
    echo "  Current: $current"
    echo "  This is the cloudflared tunnel URL for browser-side OAuth redirects"

    if confirm_action "  Change Auth9 Core Public URL?"; then
        local new_url
        while true; do
            new_url=$(prompt_user "  Auth9 Core Public URL (cloudflared tunnel)" "$current")
            if validate_url "$new_url"; then
                CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$new_url"
                break
            fi
        done
    else
        CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]="$current"
    fi

    print_success "Auth9 Core Public URL configured"
}

collect_portal_url() {
    print_info "Auth9 Portal URL Configuration"

    local current="${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.gitski.work}"
    echo "  Current: $current"
    echo "  This is the cloudflared tunnel URL for Portal"

    if confirm_action "  Change Auth9 Portal URL?"; then
        local new_url
        while true; do
            new_url=$(prompt_user "  Auth9 Portal URL (cloudflared tunnel)" "$current")
            if validate_url "$new_url"; then
                CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$new_url"
                break
            fi
        done
    else
        CONFIGMAP_VALUES[AUTH9_PORTAL_URL]="$current"
    fi

    print_success "Auth9 Portal URL configured"
}

generate_secrets() {
    # JWT_SECRET
    if [ -z "${AUTH9_SECRETS[JWT_SECRET]}" ]; then
        AUTH9_SECRETS[JWT_SECRET]=$(openssl rand -hex 32)
        echo ""
        print_warning "Generated JWT_SECRET - SAVE THIS SECURELY:"
        echo -e "${GREEN}${AUTH9_SECRETS[JWT_SECRET]}${NC}"
        echo ""
        read "?Press Enter after saving..."
    else
        print_info "JWT_SECRET already exists (not regenerating)"
    fi

    # SESSION_SECRET
    if [ -z "${AUTH9_SECRETS[SESSION_SECRET]}" ]; then
        AUTH9_SECRETS[SESSION_SECRET]=$(openssl rand -hex 32)
        echo ""
        print_warning "Generated SESSION_SECRET - SAVE THIS SECURELY:"
        echo -e "${GREEN}${AUTH9_SECRETS[SESSION_SECRET]}${NC}"
        echo ""
        read "?Press Enter after saving..."
    else
        print_info "SESSION_SECRET already exists (not regenerating)"
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
        print_info "Updating existing $secret_name..."

        for key in "${keys[@]}"; do
            local value
            eval 'value="${'$array_name'[$key]}"'
            local value_b64=$(echo -n "$value" | base64 | tr -d '\n')
            local patch_add="[{\"op\":\"add\",\"path\":\"/data/$key\",\"value\":\"$value_b64\"}]"
            local patch_replace="[{\"op\":\"replace\",\"path\":\"/data/$key\",\"value\":\"$value_b64\"}]"

            # Try add first, if it fails try replace
            if ! kubectl patch secret "$secret_name" -n "$namespace" --type=json -p="$patch_add" 2>/dev/null; then
                kubectl patch secret "$secret_name" -n "$namespace" --type=json -p="$patch_replace" 2>/dev/null || {
                    print_error "Failed to patch $key in $secret_name"
                    return 1
                }
            fi
        done

        print_success "$secret_name updated ($key_count keys)"
    else
        # Secret doesn't exist, create it
        print_info "Creating $secret_name..."

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
            print_success "$secret_name created ($key_count keys)"
        else
            print_error "Failed to create $secret_name"
            return 1
        fi
    fi
}

apply_configmap() {
    local jwt_issuer="${CONFIGMAP_VALUES[JWT_ISSUER]:-https://auth9.gitski.work}"

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
  KEYCLOAK_REALM: "auth9"
  KEYCLOAK_ADMIN_CLIENT_ID: "auth9-admin"
  AUTH9_CORE_URL: "http://auth9-core:8080"
  AUTH9_CORE_PUBLIC_URL: "${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.gitski.work}"
  AUTH9_PORTAL_URL: "${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.gitski.work}"
  NODE_ENV: "production"
EOF

    if [ $? -eq 0 ]; then
        print_success "ConfigMap applied"
    else
        print_error "Failed to apply ConfigMap"
        return 1
    fi
}

################################################################################
# Phase 5: Main Interactive Setup Flow
################################################################################

run_interactive_setup() {
    print_header "Auth9 Interactive Setup"

    # Step 1/6: Check prerequisites
    print_progress "1/6" "Checking prerequisites"
    check_prerequisites

    # Step 2/6: Detect existing configuration
    print_progress "2/6" "Detecting existing configuration"

    # Detect auth9-secrets
    detect_existing_secrets "auth9-secrets" "$NAMESPACE" AUTH9_SECRETS \
        "DATABASE_URL" "REDIS_URL" "JWT_SECRET" "SESSION_SECRET" \
        "KEYCLOAK_URL" "KEYCLOAK_ADMIN" "KEYCLOAK_ADMIN_PASSWORD" "KEYCLOAK_ADMIN_CLIENT_SECRET" || true

    # Detect keycloak-secrets
    detect_existing_secrets "keycloak-secrets" "$NAMESPACE" KEYCLOAK_SECRETS \
        "KEYCLOAK_ADMIN" "KEYCLOAK_ADMIN_PASSWORD" "KC_DB_USERNAME" "KC_DB_PASSWORD" || true

    # Detect ConfigMap
    detect_existing_configmap || true

    # Check if init job is needed
    should_run_init_job

    echo ""
    print_info "Init job needed: $([ "$NEEDS_INIT_JOB" = "true" ] && echo "yes" || echo "no (client secret exists)")"

    # Step 3/6: Collect missing configuration
    print_progress "3/6" "Collecting configuration"
    collect_database_config
    collect_redis_config
    collect_keycloak_config
    collect_keycloak_db_password
    collect_jwt_issuer
    collect_core_public_url
    collect_portal_url

    # Step 4/6: Generate secrets
    print_progress "4/6" "Generating secure secrets"
    generate_secrets

    # Step 5/6: Apply configuration
    print_progress "5/6" "Applying configuration to cluster"

    # Create namespace if it doesn't exist
    kubectl create namespace "$NAMESPACE" 2>/dev/null || true

    # Apply secrets
    create_or_patch_secret "auth9-secrets" "$NAMESPACE" AUTH9_SECRETS
    create_or_patch_secret "keycloak-secrets" "$NAMESPACE" KEYCLOAK_SECRETS

    # Apply ConfigMap
    apply_configmap

    # Step 6/6: Confirm deployment
    print_progress "6/6" "Ready to deploy"
    print_summary

    if ! confirm_action "Proceed with deployment?"; then
        print_info "Configuration saved. Run deploy.sh again to deploy."
        exit 0
    fi
}

print_summary() {
    echo ""
    echo -e "${BOLD}Configuration Summary:${NC}"
    echo "  Database: ${AUTH9_SECRETS[DATABASE_URL]%%\?*}"  # Hide password
    echo "  Redis: ${AUTH9_SECRETS[REDIS_URL]}"
    echo "  JWT Issuer: ${CONFIGMAP_VALUES[JWT_ISSUER]:-https://auth9.gitski.work}"
    echo "  Core Public URL: ${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.gitski.work}"
    echo "  Portal URL: ${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.gitski.work}"
    echo "  Init job: $([ "$NEEDS_INIT_JOB" = "true" ] && echo "will run" || echo "will skip (client secret exists)")"
    echo ""
}

################################################################################
# Phase 6: Enhanced Deployment Flow
################################################################################

deploy_auth9() {
    print_header "Auth9 Deployment"

    # Step 1: Create namespace and service account
    print_progress "1/10" "Creating namespace and service account"
    kubectl apply -f "$K8S_DIR/namespace.yaml" $DRY_RUN
    kubectl apply -f "$K8S_DIR/serviceaccount.yaml" $DRY_RUN
    kubectl apply -f "$K8S_DIR/ghcr-secret.yaml" $DRY_RUN

    # Step 2: ConfigMap already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "2/10" "Applying ConfigMap"
        kubectl apply -f "$K8S_DIR/configmap.yaml" $DRY_RUN
    else
        print_progress "2/10" "ConfigMap already applied"
    fi

    # Step 3: Secrets already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "3/10" "Checking secrets"
        check_secrets_non_interactive
    else
        print_progress "3/10" "Secrets already applied"
    fi

    # Step 4: Deploy infrastructure (keycloak, redis, postgres)
    print_progress "4/10" "Deploying infrastructure"
    deploy_infrastructure

    # Step 5-6: Wait for dependencies
    print_progress "5/10" "Waiting for keycloak-postgres to be ready"
    wait_for_postgres

    print_progress "6/10" "Waiting for keycloak to be ready"
    wait_for_keycloak

    # Step 7-8: Init job (conditional execution) - runs AFTER keycloak is ready
    if [ "$NEEDS_INIT_JOB" = "true" ] && [ -z "$SKIP_INIT" ]; then
        print_progress "7/10" "Running auth9-init job"
        run_init_job

        print_progress "8/10" "Extracting Keycloak admin client secret"
        extract_client_secret
    else
        print_progress "7/10" "Skipping auth9-init job"
        print_progress "8/10" "Skipping secret extraction"
    fi

    # Step 9: Deploy auth9 applications
    print_progress "9/10" "Deploying auth9 applications"
    deploy_auth9_apps

    # Step 10: Wait for auth9 apps to be ready
    if [ -z "$DRY_RUN" ]; then
        print_progress "10/10" "Waiting for auth9 applications"
        wait_for_auth9_apps
    else
        print_progress "10/10" "Skipping wait (dry-run)"
    fi

    print_deployment_complete
}

check_secrets_non_interactive() {
    if kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
        print_success "auth9-secrets exist"
    else
        print_warning "auth9-secrets not found. Please create them:"
        echo "    kubectl create secret generic auth9-secrets \\"
        echo "      --from-literal=DATABASE_URL='...' \\"
        echo "      --from-literal=REDIS_URL='...' \\"
        echo "      --from-literal=JWT_SECRET='...' \\"
        echo "      --from-literal=KEYCLOAK_URL='...' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN='admin' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN_PASSWORD='...' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN_CLIENT_SECRET='<will-be-auto-generated>' \\"
        echo "      --from-literal=SESSION_SECRET='...' \\"
        echo "      -n $NAMESPACE"
        echo ""
        if [ -z "$DRY_RUN" ]; then
            print_warning "Continuing anyway (deployment may fail without secrets)"
        fi
    fi

    if kubectl get secret keycloak-secrets -n "$NAMESPACE" &> /dev/null; then
        print_success "keycloak-secrets exist"
    else
        print_warning "keycloak-secrets not found. Please create them:"
        echo "    kubectl create secret generic keycloak-secrets \\"
        echo "      --from-literal=KEYCLOAK_ADMIN='admin' \\"
        echo "      --from-literal=KEYCLOAK_ADMIN_PASSWORD='...' \\"
        echo "      --from-literal=KC_DB_USERNAME='keycloak' \\"
        echo "      --from-literal=KC_DB_PASSWORD='...' \\"
        echo "      -n $NAMESPACE"
        echo ""
        if [ -z "$DRY_RUN" ]; then
            print_warning "Continuing anyway (deployment may fail without secrets)"
        fi
    fi
}

run_init_job() {
    if [ -z "$DRY_RUN" ]; then
        # Check if required secrets exist
        if ! kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
            print_error "auth9-secrets not found. Init job requires:"
            echo "    - KEYCLOAK_ADMIN"
            echo "    - KEYCLOAK_ADMIN_PASSWORD"
            echo "    - DATABASE_URL"
            echo "    - REDIS_URL"
            echo "  Please create the secret first, then run this script again."
            exit 1
        fi

        # Delete old job if exists
        if kubectl get job auth9-init -n "$NAMESPACE" &> /dev/null; then
            print_info "Deleting existing auth9-init job..."
            kubectl delete job auth9-init -n "$NAMESPACE" --ignore-not-found=true
            sleep 2
        fi

        # Apply init job
        print_info "Creating auth9-init job..."
        kubectl apply -f "$K8S_DIR/auth9-core/init-job.yaml"

        # Wait for job to complete
        print_info "Waiting for init job to complete (timeout: 300s)..."
        if kubectl wait --for=condition=complete job/auth9-init -n "$NAMESPACE" --timeout=300s 2>/dev/null; then
            print_success "Init job completed successfully"
        else
            print_error "Init job failed or timed out"
            echo ""
            echo "  Recent logs:"
            kubectl logs job/auth9-init -n "$NAMESPACE" --tail=20 2>/dev/null || true
            echo ""
            echo "  Full logs: kubectl logs job/auth9-init -n $NAMESPACE"
            exit 1
        fi
    else
        print_info "Skipping init job (dry-run)"
    fi
}

extract_client_secret() {
    if [ -z "$DRY_RUN" ]; then
        # Get the secret from init job logs
        print_info "Reading auth9-init job logs..."
        local init_logs=$(kubectl logs job/auth9-init -n "$NAMESPACE" 2>/dev/null || echo "")

        # Extract admin credentials if present
        if echo "$init_logs" | grep -q "AUTH9_ADMIN_USERNAME="; then
            AUTH9_ADMIN_USERNAME=$(echo "$init_logs" | grep "AUTH9_ADMIN_USERNAME=" | sed 's/.*AUTH9_ADMIN_USERNAME=//' | head -1)
            AUTH9_ADMIN_PASSWORD=$(echo "$init_logs" | grep "AUTH9_ADMIN_PASSWORD=" | sed 's/.*AUTH9_ADMIN_PASSWORD=//' | head -1)
            if [ -n "$AUTH9_ADMIN_PASSWORD" ]; then
                print_success "Extracted admin credentials"
            fi
        fi

        if echo "$init_logs" | grep -q "KEYCLOAK_ADMIN_CLIENT_SECRET"; then
            local client_secret=$(echo "$init_logs" | grep "KEYCLOAK_ADMIN_CLIENT_SECRET=" | sed 's/.*KEYCLOAK_ADMIN_CLIENT_SECRET=//' | head -1)

            if [ -n "$client_secret" ]; then
                print_success "Extracted client secret: ${client_secret:0:8}..."
                echo ""
                echo -e "  ${BLUE}KEYCLOAK_ADMIN_CLIENT_SECRET:${NC}"
                echo "  $client_secret"
                echo ""

                # Update auth9-secrets with the new client secret
                if kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
                    print_info "Updating auth9-secrets with new KEYCLOAK_ADMIN_CLIENT_SECRET..."
                    local client_secret_b64=$(echo -n "$client_secret" | base64 | tr -d '\n')

                    if kubectl patch secret auth9-secrets -n "$NAMESPACE" \
                        --type='json' \
                        -p="[{\"op\": \"add\", \"path\": \"/data/KEYCLOAK_ADMIN_CLIENT_SECRET\", \"value\": \"$client_secret_b64\"}]" 2>/dev/null; then
                        print_success "Secret updated successfully"
                    else
                        # Try replace if add fails
                        kubectl patch secret auth9-secrets -n "$NAMESPACE" \
                            --type='json' \
                            -p="[{\"op\": \"replace\", \"path\": \"/data/KEYCLOAK_ADMIN_CLIENT_SECRET\", \"value\": \"$client_secret_b64\"}]" 2>/dev/null || {
                            print_warning "Failed to patch secret (it may already exist)"
                            echo "  To update manually:"
                            echo "    kubectl patch secret auth9-secrets -n $NAMESPACE --type='json' \\"
                            echo "      -p='[{\"op\": \"replace\", \"path\": \"/data/KEYCLOAK_ADMIN_CLIENT_SECRET\", \"value\": \"$client_secret_b64\"}]'"
                        }
                    fi
                else
                    print_warning "auth9-secrets not found, cannot update"
                    echo "  Please manually add: KEYCLOAK_ADMIN_CLIENT_SECRET=$client_secret"
                fi
            else
                print_warning "Could not extract client secret from logs"
            fi
        else
            # Check if client already exists (idempotent operation)
            if echo "$init_logs" | grep -q "auth9-admin client already exists"; then
                print_info "auth9-admin client already exists (skipped creation)"
                echo "  If you need the secret, retrieve it manually from Keycloak Admin Console"
            else
                print_warning "No client secret found in init logs"
                echo "  This may be expected if using a preset secret or if the client already existed"
            fi
        fi
    else
        print_info "Skipping secret extraction (dry-run)"
    fi
}

deploy_infrastructure() {
    if [ -z "$DRY_RUN" ]; then
        print_info "Deploying keycloak..."
        kubectl apply -f "$K8S_DIR/keycloak/" $DRY_RUN

        print_info "Deploying redis..."
        kubectl apply -f "$K8S_DIR/redis/" $DRY_RUN

        print_success "Infrastructure deployed"
    else
        print_info "Skipping infrastructure deployment (dry-run)"
    fi
}

deploy_auth9_apps() {
    if [ -z "$DRY_RUN" ]; then
        print_info "Deploying auth9-core..."
        kubectl apply -f "$K8S_DIR/auth9-core/" $DRY_RUN

        print_info "Deploying auth9-portal..."
        kubectl apply -f "$K8S_DIR/auth9-portal/" $DRY_RUN

        print_success "Auth9 applications deployed"
    else
        print_info "Skipping auth9 deployment (dry-run)"
    fi
}

wait_for_keycloak() {
    if [ -z "$DRY_RUN" ]; then
        print_info "Waiting for keycloak deployment..."
        kubectl rollout status deployment/keycloak -n "$NAMESPACE" --timeout=300s || true

        # Wait for all keycloak pods to be ready (using kubectl wait)
        print_info "Waiting for keycloak pods to be ready..."
        if kubectl wait --for=condition=Ready pod -l app.kubernetes.io/name=keycloak -n "$NAMESPACE" --timeout=150s 2>/dev/null; then
            print_success "Keycloak is ready"
            return 0
        else
            print_warning "Keycloak readiness check timed out, continuing anyway..."
        fi
    fi
}

wait_for_auth9_apps() {
    print_info "Waiting for auth9-core..."
    kubectl rollout status deployment/auth9-core -n "$NAMESPACE" --timeout=300s || true

    print_info "Waiting for auth9-portal..."
    kubectl rollout status deployment/auth9-portal -n "$NAMESPACE" --timeout=300s || true

    print_info "Waiting for redis..."
    kubectl rollout status deployment/redis -n "$NAMESPACE" --timeout=300s || true
}

wait_for_postgres() {
    if [ -z "$DRY_RUN" ]; then
        kubectl rollout status statefulset/keycloak-postgres -n "$NAMESPACE" --timeout=300s || true
    fi
}


print_deployment_complete() {
    echo ""
    print_header "Deployment Complete!"

    if [ -z "$DRY_RUN" ]; then
        echo -e "${YELLOW}Current pod status:${NC}"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/part-of=auth9
        echo ""
        echo -e "${YELLOW}Services:${NC}"
        kubectl get svc -n "$NAMESPACE"
        echo ""
        echo -e "${YELLOW}Note:${NC} Use cloudflared to expose services. See wiki/安装部署.md"
        echo ""
        echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${CYAN}║  Cloudflared Configuration                                      ║${NC}"
        echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo -e "${BOLD}Service URLs:${NC}"
        echo ""
        local portal_url="${CONFIGMAP_VALUES[AUTH9_PORTAL_URL]:-https://auth9.gitski.work}"
        local core_url="${CONFIGMAP_VALUES[AUTH9_CORE_PUBLIC_URL]:-https://api.auth9.gitski.work}"
        echo -e "  ${GREEN}auth9-portal (Admin Dashboard):${NC}"
        echo -e "    Public URL:   ${YELLOW}${portal_url}${NC}"
        echo -e "    Internal:     auth9-portal.$NAMESPACE.svc.cluster.local:3000"
        echo ""
        echo -e "  ${GREEN}auth9-core (Backend API):${NC}"
        echo -e "    Public URL:   ${YELLOW}${core_url}${NC}"
        echo -e "    Internal:     auth9-core.$NAMESPACE.svc.cluster.local:8080"
        echo ""
        echo -e "  ${GREEN}keycloak (OIDC Provider):${NC}"
        echo -e "    Internal:     keycloak.$NAMESPACE.svc.cluster.local:8080"
        echo -e "    ${DIM}(Keycloak is accessed internally by auth9-core)${NC}"
        echo ""

        # Display admin credentials if extracted
        if [ -n "$AUTH9_ADMIN_PASSWORD" ]; then
            echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
            echo -e "${CYAN}║  Auth9 Admin Credentials                                        ║${NC}"
            echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
            echo ""
            echo -e "  ${RED}${BOLD}IMPORTANT: Save these credentials securely!${NC}"
            echo ""
            echo -e "  ${GREEN}Username:${NC}  ${YELLOW}${AUTH9_ADMIN_USERNAME}${NC}"
            echo -e "  ${GREEN}Password:${NC}  ${YELLOW}${AUTH9_ADMIN_PASSWORD}${NC}"
            echo ""
            echo -e "  ${DIM}Login at: ${portal_url}${NC}"
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
                echo -e "${RED}Unknown option: $1${NC}"
                echo ""
                echo "Usage: $0 [options]"
                echo ""
                echo "Options:"
                echo "  --interactive       Enable interactive mode (default)"
                echo "  --non-interactive   Disable interactive mode"
                echo "  --dry-run           Print what would be applied without executing"
                echo "  --skip-init         Skip the auth9-init job"
                echo "  --namespace NS      Use a different namespace (default: auth9)"
                echo "  --config-file FILE  Load configuration from file"
                exit 1
                ;;
        esac
    done
}

main() {
    parse_arguments "$@"

    # Show mode
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║         Auth9 Deployment Script            ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}Namespace:${NC} $NAMESPACE"
    echo -e "${YELLOW}K8s manifests:${NC} $K8S_DIR"
    echo -e "${YELLOW}Mode:${NC} $([ "$INTERACTIVE" = "true" ] && echo "Interactive" || echo "Non-Interactive")"
    if [ -n "$DRY_RUN" ]; then
        echo -e "${YELLOW}Dry Run:${NC} Yes"
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
