#!/bin/bash
# Stop GGEN MCP Monitoring Stack
# This script stops all monitoring services

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
echo -e "${BLUE}GGEN MCP Monitoring Stack Shutdown${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker is not running${NC}"
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

# Check if monitoring stack is running
if ! $DOCKER_COMPOSE -f docker-compose.monitoring.yml ps | grep -q "Up"; then
    echo -e "${YELLOW}Monitoring stack is not running${NC}"
    exit 0
fi

# Ask for confirmation
echo -e "${YELLOW}This will stop all monitoring services.${NC}"
read -p "Do you want to continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${BLUE}Operation cancelled${NC}"
    exit 0
fi

# Stop the monitoring stack
echo -e "${YELLOW}Stopping monitoring services...${NC}"
$DOCKER_COMPOSE -f docker-compose.monitoring.yml down

echo ""
echo -e "${GREEN}Monitoring stack stopped successfully!${NC}"
echo ""

# Ask if volumes should be removed
echo -e "${YELLOW}Do you want to remove monitoring data volumes?${NC}"
echo -e "${RED}WARNING: This will delete all metrics, logs, and dashboard data!${NC}"
read -p "Remove volumes? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Removing volumes...${NC}"
    $DOCKER_COMPOSE -f docker-compose.monitoring.yml down -v
    echo -e "${GREEN}Volumes removed${NC}"
else
    echo -e "${BLUE}Volumes preserved${NC}"
fi

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Shutdown Complete${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${BLUE}To start the monitoring stack again:${NC}"
echo -e "  ./scripts/start-monitoring.sh"
echo ""

exit 0
