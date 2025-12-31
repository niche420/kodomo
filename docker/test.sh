set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}🧪 Testing Docker Deployment${NC}"
echo ""

# Start services in background
echo "Starting services..."
docker-compose up -d

# Wait for services to be healthy
echo "Waiting for services to start..."
sleep 10

# Test 1: Check containers are running
echo -n "Test 1: Containers running... "
if docker-compose ps | grep -q "Up"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    exit 1
fi

# Test 2: Server responding
echo -n "Test 2: Server responding... "
if docker-compose logs streaming-server | grep -q "Streaming server is running"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    exit 1
fi

# Test 3: ML optimizer connected
echo -n "Test 3: ML optimizer connected... "
if docker-compose logs ml-optimizer | grep -q "Starting ML Optimizer"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    exit 1
fi

# Test 4: Prometheus scraping
echo -n "Test 4: Prometheus accessible... "
if curl -s http://localhost:9091/-/healthy | grep -q "Prometheus"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    exit 1
fi

# Test 5: Grafana accessible
echo -n "Test 5: Grafana accessible... "
if curl -s http://localhost:3000/api/health | grep -q "ok"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}✅ All tests passed!${NC}"
echo ""
echo "Services running:"
docker-compose ps
echo ""
echo "Access:"
echo "  Grafana: http://localhost:3000 (admin/admin)"
echo "  Prometheus: http://localhost:9091"
echo ""
echo "Stop with: docker-compose down"