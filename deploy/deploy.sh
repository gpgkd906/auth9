#!/bin/bash
# Auth9 Deployment Script
# 
# This script deploys Auth9 to a Kubernetes cluster.
# 
# Usage:
#   ./deploy.sh [options]
#
# Options:
#   --dry-run       Print what would be applied without executing
#   --skip-restart  Skip the deployment restart step
#   --namespace NS  Use a different namespace (default: auth9)
#
# Prerequisites:
#   - kubectl configured with cluster access
#   - Secrets must be created separately (see secrets.yaml.example)

set -e

# Configuration
NAMESPACE="${NAMESPACE:-auth9}"
K8S_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/k8s"
DRY_RUN=""
SKIP_RESTART=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN="--dry-run=client"
            shift
            ;;
        --skip-restart)
            SKIP_RESTART="true"
            shift
            ;;
        --namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Auth9 Deployment Script            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Namespace:${NC} $NAMESPACE"
echo -e "${YELLOW}K8s manifests:${NC} $K8S_DIR"
if [ -n "$DRY_RUN" ]; then
    echo -e "${YELLOW}Mode:${NC} Dry Run"
fi
echo ""

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}Error: kubectl is not installed${NC}"
    exit 1
fi

# Check cluster access
if ! kubectl cluster-info &> /dev/null; then
    echo -e "${RED}Error: Cannot connect to Kubernetes cluster${NC}"
    exit 1
fi

# Step 1: Create namespace and service account
echo -e "${GREEN}[1/5] Creating namespace and service account...${NC}"
kubectl apply -f "$K8S_DIR/namespace.yaml" $DRY_RUN
kubectl apply -f "$K8S_DIR/serviceaccount.yaml" $DRY_RUN

# Step 2: Apply ConfigMap
echo -e "${GREEN}[2/5] Applying ConfigMap...${NC}"
kubectl apply -f "$K8S_DIR/configmap.yaml" $DRY_RUN

# Step 3: Check for secrets
echo -e "${GREEN}[3/5] Checking secrets...${NC}"
if kubectl get secret auth9-secrets -n "$NAMESPACE" &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} Secrets exist"
else
    echo -e "  ${YELLOW}⚠ Secrets not found. Please create them:${NC}"
    echo "    kubectl create secret generic auth9-secrets \\"
    echo "      --from-literal=DATABASE_URL='...' \\"
    echo "      --from-literal=REDIS_URL='...' \\"
    echo "      --from-literal=JWT_SECRET='...' \\"
    echo "      --from-literal=KEYCLOAK_URL='...' \\"
    echo "      --from-literal=KEYCLOAK_ADMIN_CLIENT_SECRET='...' \\"
    echo "      --from-literal=SESSION_SECRET='...' \\"
    echo "      -n $NAMESPACE"
    echo ""
    if [ -z "$DRY_RUN" ]; then
        echo -e "  ${YELLOW}Continuing anyway (deployment may fail without secrets)${NC}"
    fi
fi

# Step 4: Deploy auth9-core and auth9-portal
echo -e "${GREEN}[4/5] Deploying applications...${NC}"

echo "  Deploying auth9-core..."
kubectl apply -f "$K8S_DIR/auth9-core/" $DRY_RUN

echo "  Deploying auth9-portal..."
kubectl apply -f "$K8S_DIR/auth9-portal/" $DRY_RUN

echo "  Applying ingress..."
kubectl apply -f "$K8S_DIR/ingress.yaml" $DRY_RUN

# Step 5: Restart deployments (if not skipped and not dry-run)
if [ -z "$SKIP_RESTART" ] && [ -z "$DRY_RUN" ]; then
    echo -e "${GREEN}[5/5] Restarting deployments to pick up latest images...${NC}"
    
    echo "  Restarting auth9-core..."
    kubectl rollout restart deployment/auth9-core -n "$NAMESPACE"
    
    echo "  Restarting auth9-portal..."
    kubectl rollout restart deployment/auth9-portal -n "$NAMESPACE"
    
    echo ""
    echo -e "${YELLOW}Waiting for rollout to complete...${NC}"
    
    echo "  Waiting for auth9-core..."
    kubectl rollout status deployment/auth9-core -n "$NAMESPACE" --timeout=300s
    
    echo "  Waiting for auth9-portal..."
    kubectl rollout status deployment/auth9-portal -n "$NAMESPACE" --timeout=300s
else
    echo -e "${GREEN}[5/5] Skipping restart step${NC}"
fi

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Deployment Complete!               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

# Show status
if [ -z "$DRY_RUN" ]; then
    echo -e "${YELLOW}Current pod status:${NC}"
    kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/part-of=auth9
    echo ""
    echo -e "${YELLOW}Services:${NC}"
    kubectl get svc -n "$NAMESPACE"
    echo ""
    echo -e "${YELLOW}Ingress:${NC}"
    kubectl get ingress -n "$NAMESPACE"
fi
