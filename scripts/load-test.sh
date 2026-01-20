#!/bin/bash
# Load Test Script for GGEN MCP Server
# Generates test traffic to validate monitoring and alert configuration

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
MCP_HOST="${MCP_HOST:-localhost}"
MCP_PORT="${MCP_PORT:-9464}"
MCP_BASE_URL="http://${MCP_HOST}:${MCP_PORT}"
DURATION="${DURATION:-60}"  # Duration in seconds
CONCURRENT="${CONCURRENT:-5}"  # Concurrent requests

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}GGEN MCP Load Test${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${BLUE}Target:${NC} ${MCP_BASE_URL}"
echo -e "${BLUE}Duration:${NC} ${DURATION}s"
echo -e "${BLUE}Concurrency:${NC} ${CONCURRENT}"
echo ""

# Check if service is reachable
if ! curl -s -f "${MCP_BASE_URL}/health" > /dev/null 2>&1; then
    echo -e "${RED}Error: Cannot reach GGEN MCP service at ${MCP_BASE_URL}${NC}"
    echo "Please ensure the service is running"
    exit 1
fi

# Function to make a request
make_request() {
    local endpoint=$1
    local method=${2:-GET}
    local data=${3:-}

    if [ -n "$data" ]; then
        curl -s -X "${method}" \
             -H "Content-Type: application/json" \
             -d "${data}" \
             "${MCP_BASE_URL}${endpoint}" > /dev/null 2>&1
    else
        curl -s -X "${method}" "${MCP_BASE_URL}${endpoint}" > /dev/null 2>&1
    fi

    return $?
}

# Function to simulate normal traffic
normal_traffic() {
    local end_time=$((SECONDS + DURATION))
    local request_count=0

    echo -e "${GREEN}Simulating normal traffic...${NC}"

    while [ $SECONDS -lt $end_time ]; do
        # Mix of different operations
        case $((RANDOM % 10)) in
            0|1|2|3)  # 40% - Health checks
                make_request "/health" "GET"
                ;;
            4|5|6)    # 30% - Metrics
                make_request "/metrics" "GET"
                ;;
            7|8)      # 20% - Query operations (simulated)
                make_request "/api/query" "POST" '{"query":"SELECT * FROM data"}'
                ;;
            9)        # 10% - Other operations
                make_request "/api/status" "GET"
                ;;
        esac

        ((request_count++))

        # Small delay between requests (100-500ms)
        sleep 0.$((RANDOM % 5))
    done

    echo -e "${GREEN}Completed ${request_count} requests${NC}"
}

# Function to simulate high load
high_load_traffic() {
    local end_time=$((SECONDS + DURATION))
    local request_count=0

    echo -e "${YELLOW}Simulating high load traffic...${NC}"

    # Spawn multiple background processes
    for i in $(seq 1 $CONCURRENT); do
        (
            while [ $SECONDS -lt $end_time ]; do
                make_request "/health" "GET"
                make_request "/metrics" "GET"
                ((request_count++))
            done
        ) &
    done

    wait

    echo -e "${YELLOW}Completed high load test${NC}"
}

# Function to simulate error conditions
error_traffic() {
    echo -e "${RED}Simulating error conditions...${NC}"

    # Invalid endpoints
    for i in {1..10}; do
        curl -s "${MCP_BASE_URL}/invalid-endpoint-${i}" > /dev/null 2>&1 || true
        sleep 0.5
    done

    # Malformed requests
    for i in {1..10}; do
        curl -s -X POST \
             -H "Content-Type: application/json" \
             -d "invalid json {{{" \
             "${MCP_BASE_URL}/api/query" > /dev/null 2>&1 || true
        sleep 0.5
    done

    echo -e "${RED}Completed error simulation${NC}"
}

# Function to simulate cache operations
cache_traffic() {
    echo -e "${BLUE}Simulating cache operations...${NC}"

    # Repeated requests to test cache hits
    for i in {1..50}; do
        make_request "/api/query" "POST" '{"query":"SELECT * FROM cached_data"}'

        if [ $((i % 10)) -eq 0 ]; then
            sleep 1
        else
            sleep 0.1
        fi
    done

    echo -e "${BLUE}Completed cache simulation${NC}"
}

# Function to display statistics
show_statistics() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}Load Test Statistics${NC}"
    echo -e "${BLUE}========================================${NC}"

    # Query Prometheus for statistics
    if command -v curl &> /dev/null; then
        echo -e "${YELLOW}Fetching metrics from Prometheus...${NC}"

        # Total requests
        total_requests=$(curl -s "http://localhost:9090/api/v1/query?query=sum(increase(ggen_mcp_requests_total[1m]))" | grep -o '"result":\[{"value":\[[0-9.]*,"[0-9.]*"\]' | grep -o '[0-9.]*"$' | tr -d '"' || echo "0")
        echo -e "${BLUE}Total Requests (last 1m):${NC} ${total_requests}"

        # Error rate
        error_rate=$(curl -s "http://localhost:9090/api/v1/query?query=rate(ggen_mcp_errors_total[1m])" | grep -o '"result":\[{"value":\[[0-9.]*,"[0-9.]*"\]' | grep -o '[0-9.]*"$' | tr -d '"' || echo "0")
        echo -e "${BLUE}Error Rate (last 1m):${NC} ${error_rate}/sec"

        # Cache hit rate
        cache_hit_rate=$(curl -s "http://localhost:9090/api/v1/query?query=ggen_mcp:cache_hit_rate_percentage:5m" | grep -o '"result":\[{"value":\[[0-9.]*,"[0-9.]*"\]' | grep -o '[0-9.]*"$' | tr -d '"' || echo "0")
        echo -e "${BLUE}Cache Hit Rate (last 5m):${NC} ${cache_hit_rate}%"
    fi

    echo ""
    echo -e "${GREEN}View detailed metrics at:${NC}"
    echo -e "  Grafana: ${GREEN}http://localhost:3000/d/ggen-mcp-prod${NC}"
    echo -e "  Prometheus: ${GREEN}http://localhost:9090${NC}"
    echo ""
}

# Main menu
echo -e "${BLUE}Select load test scenario:${NC}"
echo "  1) Normal traffic (${DURATION}s)"
echo "  2) High load traffic (${DURATION}s)"
echo "  3) Error conditions"
echo "  4) Cache operations"
echo "  5) Full test suite (all scenarios)"
echo "  6) Custom duration"
echo ""
read -p "Enter choice (1-6): " choice

case $choice in
    1)
        normal_traffic
        ;;
    2)
        high_load_traffic
        ;;
    3)
        error_traffic
        ;;
    4)
        cache_traffic
        ;;
    5)
        echo -e "${BLUE}Running full test suite...${NC}"
        echo ""
        normal_traffic
        sleep 5
        high_load_traffic
        sleep 5
        error_traffic
        sleep 5
        cache_traffic
        ;;
    6)
        read -p "Enter duration (seconds): " DURATION
        read -p "Enter concurrency: " CONCURRENT
        normal_traffic
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

# Show statistics
show_statistics

echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Load Test Complete${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

exit 0
