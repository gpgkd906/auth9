#!/bin/bash
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
#   --skip-restart      Skip the deployment restart step
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
K8S_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/k8s"
DRY_RUN=""
SKIP_RESTART=""
SKIP_INIT=""
INTERACTIVE="true"
CONFIG_FILE=""
NEEDS_INIT_JOB="false"

# Associative arrays for configuration
declare -A AUTH9_SECRETS
declare -A KEYCLOAK_SECRETS
declare -A CONFIGMAP_VALUES

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
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
        read -p "$message [$default]: " input
        echo "${input:-$default}"
    else
        read -p "$message: " input
        echo "$input"
    fi
}

prompt_password() {
    local message="$1"
    local pass1
    local pass2

    while true; do
        read -s -p "$message: " pass1
        echo ""
        read -s -p "Confirm password: " pass2
        echo ""

        if [ "$pass1" = "$pass2" ] && [ -n "$pass1" ]; then
            echo "$pass1"
            return 0
        fi

        print_error "Passwords don't match or empty. Please try again."
    done
}

confirm_action() {
    local message="$1"
    local response

    while true; do
        read -p "$message [y/N]: " response
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
    local -n secret_array=$3
    local keys=("${!4}")

    if ! kubectl get secret "$secret_name" -n "$namespace" &>/dev/null; then
        print_warning "$secret_name not found (will create)"
        return 1
    fi

    local found_count=0
    for key in "${keys[@]}"; do
        local value=$(kubectl get secret "$secret_name" -n "$namespace" -o jsonpath="{.data.$key}" 2>/dev/null | base64 -d 2>/dev/null || echo "")
        if [ -n "$value" ]; then
            secret_array[$key]="$value"
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

    # Read JWT_ISSUER
    local jwt_issuer=$(kubectl get configmap auth9-config -n "$NAMESPACE" -o jsonpath='{.data.JWT_ISSUER}' 2>/dev/null || echo "")
    if [ -n "$jwt_issuer" ]; then
        CONFIGMAP_VALUES[JWT_ISSUER]="$jwt_issuer"
        print_info "auth9-config ConfigMap found (JWT_ISSUER: $jwt_issuer)"
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

    local current="${CONFIGMAP_VALUES[JWT_ISSUER]:-https://auth9.example.com}"
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

generate_secrets() {
    # JWT_SECRET
    if [ -z "${AUTH9_SECRETS[JWT_SECRET]}" ]; then
        AUTH9_SECRETS[JWT_SECRET]=$(openssl rand -hex 32)
        echo ""
        print_warning "Generated JWT_SECRET - SAVE THIS SECURELY:"
        echo -e "${GREEN}${AUTH9_SECRETS[JWT_SECRET]}${NC}"
        echo ""
        read -p "Press Enter after saving..."
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
        read -p "Press Enter after saving..."
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
    local -n secret_data=$3

    if kubectl get secret "$secret_name" -n "$namespace" &>/dev/null; then
        # Secret exists, use patch
        print_info "Updating existing $secret_name..."

        for key in "${!secret_data[@]}"; do
            local value_b64=$(echo -n "${secret_data[$key]}" | base64 | tr -d '\n')
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

        print_success "$secret_name updated (${#secret_data[@]} keys)"
    else
        # Secret doesn't exist, create it
        print_info "Creating $secret_name..."

        local create_cmd="kubectl create secret generic $secret_name"
        for key in "${!secret_data[@]}"; do
            # Escape single quotes in value
            local escaped_value="${secret_data[$key]//\'/\'\\\'\'}"
            create_cmd+=" --from-literal=$key='${escaped_value}'"
        done
        create_cmd+=" -n $namespace"

        if eval "$create_cmd"; then
            print_success "$secret_name created (${#secret_data[@]} keys)"
        else
            print_error "Failed to create $secret_name"
            return 1
        fi
    fi
}

apply_configmap() {
    local jwt_issuer="${CONFIGMAP_VALUES[JWT_ISSUER]:-https://auth9.example.com}"

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
    local auth9_keys=("DATABASE_URL" "REDIS_URL" "JWT_SECRET" "SESSION_SECRET" "KEYCLOAK_URL" "KEYCLOAK_ADMIN" "KEYCLOAK_ADMIN_PASSWORD" "KEYCLOAK_ADMIN_CLIENT_SECRET")
    detect_existing_secrets "auth9-secrets" "$NAMESPACE" AUTH9_SECRETS auth9_keys[@]

    # Detect keycloak-secrets
    local keycloak_keys=("KEYCLOAK_ADMIN" "KEYCLOAK_ADMIN_PASSWORD" "KC_DB_USERNAME" "KC_DB_PASSWORD")
    detect_existing_secrets "keycloak-secrets" "$NAMESPACE" KEYCLOAK_SECRETS keycloak_keys[@]

    # Detect ConfigMap
    detect_existing_configmap

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
    echo "  JWT Issuer: ${CONFIGMAP_VALUES[JWT_ISSUER]:-https://auth9.example.com}"
    echo "  Init job: $([ "$NEEDS_INIT_JOB" = "true" ] && echo "will run" || echo "will skip (client secret exists)")"
    echo ""
}

################################################################################
# Phase 6: Enhanced Deployment Flow
################################################################################

deploy_auth9() {
    print_header "Auth9 Deployment"

    # Step 1: Create namespace and service account
    print_progress "1/9" "Creating namespace and service account"
    kubectl apply -f "$K8S_DIR/namespace.yaml" $DRY_RUN
    kubectl apply -f "$K8S_DIR/serviceaccount.yaml" $DRY_RUN

    # Step 2: ConfigMap already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "2/9" "Applying ConfigMap"
        kubectl apply -f "$K8S_DIR/configmap.yaml" $DRY_RUN
    else
        print_progress "2/9" "ConfigMap already applied"
    fi

    # Step 3: Secrets already applied in interactive setup (skip if interactive)
    if [ "$INTERACTIVE" != "true" ]; then
        print_progress "3/9" "Checking secrets"
        check_secrets_non_interactive
    else
        print_progress "3/9" "Secrets already applied"
    fi

    # Step 4-5: Init job (conditional execution)
    if [ "$NEEDS_INIT_JOB" = "true" ] && [ -z "$SKIP_INIT" ]; then
        print_progress "4/9" "Running auth9-init job"
        run_init_job

        print_progress "5/9" "Extracting Keycloak admin client secret"
        extract_client_secret
    else
        print_progress "4/9" "Skipping auth9-init job"
        print_progress "5/9" "Skipping secret extraction"
    fi

    # Step 6: Deploy applications
    print_progress "6/9" "Deploying applications"
    deploy_applications

    # Step 7-8: Wait for dependencies
    print_progress "7/9" "Waiting for keycloak-postgres to be ready"
    wait_for_postgres

    print_progress "8/9" "Waiting for redis to be ready"
    wait_for_redis

    # Step 9: Restart deployments
    if [ -z "$SKIP_RESTART" ] && [ -z "$DRY_RUN" ]; then
        print_progress "9/9" "Restarting deployments"
        restart_deployments
    else
        print_progress "9/9" "Skipping restart step"
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

        if echo "$init_logs" | grep -q "KEYCLOAK_ADMIN_CLIENT_SECRET"; then
            local client_secret=$(echo "$init_logs" | grep "KEYCLOAK_ADMIN_CLIENT_SECRET=" | sed 's/.*KEYCLOAK_ADMIN_CLIENT_SECRET=//' | head -1)

            if [ -n "$client_secret" ]; then
                print_success "Extracted client secret: ${client_secret:0:8}..."
                echo ""
                echo -e "  ${BLUE}📋 KEYCLOAK_ADMIN_CLIENT_SECRET:${NC}"
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

deploy_applications() {
    if [ -z "$DRY_RUN" ]; then
        print_info "Deploying auth9-core..."
        kubectl apply -f "$K8S_DIR/auth9-core/" $DRY_RUN

        print_info "Deploying auth9-portal..."
        kubectl apply -f "$K8S_DIR/auth9-portal/" $DRY_RUN

        print_info "Deploying keycloak..."
        kubectl apply -f "$K8S_DIR/keycloak/" $DRY_RUN

        print_info "Deploying redis..."
        kubectl apply -f "$K8S_DIR/redis/" $DRY_RUN

        print_success "All applications deployed"
    else
        print_info "Skipping application deployment (dry-run)"
    fi
}

wait_for_postgres() {
    if [ -z "$DRY_RUN" ]; then
        kubectl rollout status statefulset/keycloak-postgres -n "$NAMESPACE" --timeout=300s || true
    fi
}

wait_for_redis() {
    if [ -z "$DRY_RUN" ]; then
        kubectl rollout status deployment/redis -n "$NAMESPACE" --timeout=300s || true
    fi
}

restart_deployments() {
    print_info "Restarting auth9-core..."
    kubectl rollout restart deployment/auth9-core -n "$NAMESPACE"

    print_info "Restarting auth9-portal..."
    kubectl rollout restart deployment/auth9-portal -n "$NAMESPACE"

    print_info "Restarting keycloak..."
    kubectl rollout restart deployment/keycloak -n "$NAMESPACE"

    print_info "Restarting redis..."
    kubectl rollout restart deployment/redis -n "$NAMESPACE"

    echo ""
    print_info "Waiting for rollout to complete..."

    print_info "Waiting for auth9-core..."
    kubectl rollout status deployment/auth9-core -n "$NAMESPACE" --timeout=300s

    print_info "Waiting for auth9-portal..."
    kubectl rollout status deployment/auth9-portal -n "$NAMESPACE" --timeout=300s

    print_info "Waiting for keycloak..."
    kubectl rollout status deployment/keycloak -n "$NAMESPACE" --timeout=300s

    print_info "Waiting for redis..."
    kubectl rollout status deployment/redis -n "$NAMESPACE" --timeout=300s
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
            --skip-restart)
                SKIP_RESTART="true"
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
                echo "  --skip-restart      Skip the deployment restart step"
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
