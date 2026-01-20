#!/bin/bash
# Start GGEN MCP Monitoring Stack
# This script starts Prometheus, Grafana, Alertmanager, Loki, and related services

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}GGEN MCP Monitoring Stack Startup${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker is not running${NC}"
    echo "Please start Docker and try again"
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose &> /dev/null; then
    echo -e "${YELLOW}Warning: docker-compose command not found, trying 'docker compose'${NC}"
    DOCKER_COMPOSE="docker compose"
else
    DOCKER_COMPOSE="docker-compose"
fi

# Change to project root
cd "${PROJECT_ROOT}"

# Create required directories
echo -e "${YELLOW}Creating required directories...${NC}"
mkdir -p prometheus/alerts
mkdir -p prometheus/rules
mkdir -p grafana/dashboards
mkdir -p grafana/datasources
mkdir -p alertmanager
mkdir -p loki
mkdir -p promtail
mkdir -p blackbox

# Check if .env file exists, create if not
if [ ! -f .env.monitoring ]; then
    echo -e "${YELLOW}Creating .env.monitoring file...${NC}"
    cat > .env.monitoring << 'EOF'
# Grafana Configuration
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=admin

# Alertmanager Configuration
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
SMTP_USERNAME=alerts@example.com
SMTP_PASSWORD=changeme
PAGERDUTY_SERVICE_KEY=your-pagerduty-key
ONCALL_EMAIL=oncall@example.com

# Postgres Configuration (optional)
# POSTGRES_USER=ggen_mcp
# POSTGRES_PASSWORD=changeme
EOF
    echo -e "${GREEN}Created .env.monitoring file. Please update with your actual credentials.${NC}"
fi

# Load environment variables
if [ -f .env.monitoring ]; then
    echo -e "${YELLOW}Loading environment variables...${NC}"
    set -a
    source .env.monitoring
    set +a
fi

# Stop any existing monitoring stack
echo -e "${YELLOW}Stopping any existing monitoring services...${NC}"
$DOCKER_COMPOSE -f docker-compose.monitoring.yml down 2>/dev/null || true

# Pull latest images
echo -e "${YELLOW}Pulling latest Docker images...${NC}"
$DOCKER_COMPOSE -f docker-compose.monitoring.yml pull

# Start the monitoring stack
echo -e "${GREEN}Starting monitoring stack...${NC}"
$DOCKER_COMPOSE -f docker-compose.monitoring.yml up -d

# Wait for services to be healthy
echo -e "${YELLOW}Waiting for services to become healthy...${NC}"
sleep 10

# Check service health
echo ""
echo -e "${BLUE}Service Health Status:${NC}"
echo "-----------------------------------"

check_service() {
    local service=$1
    local port=$2
    local endpoint=$3

    if curl -s -f "http://localhost:${port}${endpoint}" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} ${service} (http://localhost:${port})"
    else
        echo -e "${RED}✗${NC} ${service} (http://localhost:${port}) - Not responding"
    fi
}

check_service "Prometheus" "9090" "/-/healthy"
check_service "Grafana" "3000" "/api/health"
check_service "Alertmanager" "9093" "/-/healthy"
check_service "Loki" "3100" "/ready"
check_service "Jaeger" "16686" "/"
check_service "Node Exporter" "9100" "/metrics"
check_service "cAdvisor" "8080" "/healthz"

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Monitoring Stack Started Successfully!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${BLUE}Access URLs:${NC}"
echo -e "  Grafana:        ${GREEN}http://localhost:3000${NC} (admin/admin)"
echo -e "  Prometheus:     ${GREEN}http://localhost:9090${NC}"
echo -e "  Alertmanager:   ${GREEN}http://localhost:9093${NC}"
echo -e "  Jaeger UI:      ${GREEN}http://localhost:16686${NC}"
echo -e "  Loki:           ${GREEN}http://localhost:3100${NC}"
echo ""
echo -e "${BLUE}Grafana Dashboards:${NC}"
echo -e "  Main Dashboard: ${GREEN}http://localhost:3000/d/ggen-mcp-prod${NC}"
echo -e "  Cache Dashboard: ${GREEN}http://localhost:3000/d/ggen-mcp-cache${NC}"
echo ""
echo -e "${YELLOW}Default Grafana credentials: admin / admin${NC}"
echo -e "${YELLOW}You will be prompted to change the password on first login${NC}"
echo ""
echo -e "${BLUE}To view logs:${NC}"
echo -e "  docker-compose -f docker-compose.monitoring.yml logs -f [service-name]"
echo ""
echo -e "${BLUE}To stop the monitoring stack:${NC}"
echo -e "  ./scripts/stop-monitoring.sh"
echo ""

# Optional: Open Grafana in browser
if command -v open &> /dev/null; then
    read -p "Open Grafana in browser? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        open "http://localhost:3000"
    fi
elif command -v xdg-open &> /dev/null; then
    read -p "Open Grafana in browser? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        xdg-open "http://localhost:3000"
    fi
fi

exit 0
