set -e

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Building Docker Images${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}Error: Docker not installed${NC}"
    exit 1
fi

# Check if docker-compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo -e "${YELLOW}Error: docker-compose not installed${NC}"
    exit 1
fi

# Build Rust first (needed for FFI)
echo -e "${YELLOW}[1/4] Building Rust workspace...${NC}"
cd ..
cargo build --release --workspace
echo -e "${GREEN}✓ Rust built${NC}"
echo ""

# Build server image
echo -e "${YELLOW}[2/4] Building server image...${NC}"
docker build -f docker/Dockerfile.server -t streaming-server:latest .
echo -e "${GREEN}✓ Server image built${NC}"
echo ""

# Build ML optimizer image
echo -e "${YELLOW}[3/4] Building ML optimizer image...${NC}"
docker build -f docker/Dockerfile.ml-optimizer -t ml-optimizer:latest .
echo -e "${GREEN}✓ ML optimizer image built${NC}"
echo ""

# Build client image (optional)
echo -e "${YELLOW}[4/4] Building client image...${NC}"
docker build -f docker/Dockerfile.client -t streaming-client:latest .
echo -e "${GREEN}✓ Client image built${NC}"
echo ""

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  ✅ All images built successfully!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Images:"
docker images | grep streaming
echo ""