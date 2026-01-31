#!/usr/bin/env bash
#
# Auth9 性能测试脚本
# 用法: ./scripts/benchmark.sh [quick|full]
#

set -e

# 配置
BASE_URL="${BASE_URL:-http://localhost:8080}"
MODE="${1:-quick}"

# 颜色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

# 结果变量
HEALTH_MAX_QPS=0
HEALTH_BEST_C=0
HEALTH_P99=""
READY_MAX_QPS=0
READY_BEST_C=0
READY_P99=""
TENANTS_MAX_QPS=0
TENANTS_BEST_C=0
TENANTS_P99=""

print_header() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}  $1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
}

check_service() {
    print_header "检查服务状态"

    if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
        echo -e "${RED}[x] 服务未运行，请先启动 auth9-core${NC}"
        echo "  cd auth9-core && cargo run --release"
        exit 1
    fi

    local health=$(curl -s "$BASE_URL/health")
    echo -e "${GREEN}[ok] 服务运行中${NC}"
    echo "  $health"
}

# 找最大稳定QPS（逐步增加并发）
find_max_qps() {
    local name="$1"
    local url="$2"
    local duration="$3"

    echo -e "\n${CYAN}> $name${NC}"
    echo "  URL: $url"
    echo ""

    local max_qps=0
    local best_concurrency=0
    local last_p99=""

    # 根据模式选择并发级别
    local concurrencies
    if [ "$MODE" = "quick" ]; then
        concurrencies="50 100 200"
    else
        concurrencies="50 100 200 500 1000 2000"
    fi

    for c in $concurrencies; do
        local requests=$((c * duration))
        local output=$(hey -n "$requests" -c "$c" -q 0 "$url" 2>&1)

        local qps=$(echo "$output" | grep "Requests/sec:" | awk '{print $2}' | cut -d. -f1)
        # hey格式: "50% in 0.0009 secs" -> 提取秒数并转换为ms
        local p50_sec=$(echo "$output" | grep "50%" | awk '{print $3}')
        local p99_sec=$(echo "$output" | grep "99%" | awk '{print $3}')
        local p50=$(echo "$p50_sec" | awk '{printf "%.2fms", $1 * 1000}')
        local p99=$(echo "$p99_sec" | awk '{printf "%.2fms", $1 * 1000}')

        printf "  %-12s QPS: %-8s  P50: %-10s  P99: %-10s" "c=$c" "$qps" "$p50" "$p99"

        # 判断是否达到瓶颈
        if [ -n "$qps" ] && [ "$qps" -gt "$max_qps" ] 2>/dev/null; then
            max_qps=$qps
            best_concurrency=$c
            echo ""
        else
            if [ -n "$qps" ] && [ "$qps" -lt "$max_qps" ] 2>/dev/null; then
                echo -e " ${YELLOW}<- 性能下降${NC}"
            else
                echo ""
            fi
        fi

        last_p99="$p99"
    done

    # 存储结果到对应变量
    case "$name" in
        *health*)
            HEALTH_MAX_QPS=$max_qps
            HEALTH_BEST_C=$best_concurrency
            HEALTH_P99="$last_p99"
            ;;
        *ready*|*Ready*)
            READY_MAX_QPS=$max_qps
            READY_BEST_C=$best_concurrency
            READY_P99="$last_p99"
            ;;
        *tenant*|*Tenant*|*API*)
            TENANTS_MAX_QPS=$max_qps
            TENANTS_BEST_C=$best_concurrency
            TENANTS_P99="$last_p99"
            ;;
    esac
}

# 主测试流程
main() {
    echo ""
    echo -e "${BOLD}+---------------------------------------------------------------+${NC}"
    echo -e "${BOLD}|           Auth9 性能基准测试 (Rust + Axum)                    |${NC}"
    echo -e "${BOLD}+---------------------------------------------------------------+${NC}"
    echo ""
    echo -e "  模式: ${CYAN}$MODE${NC}  (使用 'full' 参数进行完整测试)"
    echo -e "  目标: ${CYAN}$BASE_URL${NC}"

    # 检查服务
    check_service

    # 预热
    print_header "预热服务"
    echo "  发送 1000 请求预热..."
    hey -n 1000 -c 50 -q 0 "$BASE_URL/health" > /dev/null 2>&1
    echo -e "  ${GREEN}[ok] 预热完成${NC}"

    # 测试1: 健康检查（纯Rust性能基线）
    print_header "测试 1/3: 健康检查端点 (纯计算性能)"
    find_max_qps "health" "$BASE_URL/health" 10

    # 测试2: Ready检查（包含DB+Redis）
    print_header "测试 2/3: Ready端点 (DB + Redis)"
    find_max_qps "ready" "$BASE_URL/ready" 10

    # 测试3: API端点
    print_header "测试 3/3: API端点 (业务逻辑)"
    find_max_qps "tenants" "$BASE_URL/api/v1/tenants?limit=10" 10

    # 输出最终报告
    print_header "测试结果汇总"
    echo ""
    echo -e "  ${BOLD}端点                    最大稳定QPS      最佳并发      P99延迟${NC}"
    echo "  -------------------------------------------------------------------"
    printf "  %-22s  ${GREEN}%-14s${NC}  %-12s  %s\n" \
        "/health (纯计算)" "$HEALTH_MAX_QPS" "$HEALTH_BEST_C" "$HEALTH_P99"
    printf "  %-22s  ${GREEN}%-14s${NC}  %-12s  %s\n" \
        "/ready (DB+Redis)" "$READY_MAX_QPS" "$READY_BEST_C" "$READY_P99"
    printf "  %-22s  ${GREEN}%-14s${NC}  %-12s  %s\n" \
        "/api/v1/tenants" "$TENANTS_MAX_QPS" "$TENANTS_BEST_C" "$TENANTS_P99"
    echo ""

    # 性能评估
    print_header "性能评估"
    echo ""
    if [ "$HEALTH_MAX_QPS" -gt 30000 ] 2>/dev/null; then
        echo -e "  ${GREEN}[优秀]${NC} 纯计算性能 > 30,000 QPS"
        echo "    Rust + Axum 技术栈选择正确，性能表现出色"
    elif [ "$HEALTH_MAX_QPS" -gt 10000 ] 2>/dev/null; then
        echo -e "  ${GREEN}[良好]${NC} 纯计算性能 > 10,000 QPS"
        echo "    性能符合预期，可满足大多数生产环境需求"
    elif [ "$HEALTH_MAX_QPS" -gt 5000 ] 2>/dev/null; then
        echo -e "  ${YELLOW}[一般]${NC} 纯计算性能 > 5,000 QPS"
        echo "    建议检查是否有性能瓶颈"
    else
        echo -e "  ${RED}[偏低]${NC} 纯计算性能 < 5,000 QPS"
        echo "    可能存在问题，建议使用 release 模式运行"
    fi
    echo ""

    # 对比参考
    echo -e "  ${BOLD}参考对比 (同类技术栈):${NC}"
    echo "  +-- Node.js/Express:  ~5,000 - 15,000 QPS"
    echo "  +-- Go/Gin:           ~20,000 - 50,000 QPS"
    echo "  +-- Rust/Axum:        ~30,000 - 100,000+ QPS"
    echo "  +-- 你的结果:         ~$HEALTH_MAX_QPS QPS"
    echo ""

    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "  测试完成! 使用 ${CYAN}./scripts/benchmark.sh full${NC} 运行完整测试"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

# 运行
main
