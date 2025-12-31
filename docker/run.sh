set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🎮 Starting Game Streaming Stack${NC}"
echo ""

# Parse arguments
GPU=false
DETACH=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --gpu)
            GPU=true
            shift
            ;;
        -d|--detach)
            DETACH=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./run.sh [--gpu] [-d|--detach]"
            exit 1
            ;;
    esac
done

# Build images if needed
if [[ "$(docker images -q streaming-server:latest 2> /dev/null)" == "" ]]; then
    echo -e "${YELLOW}Images not found, building...${NC}"
    ./build.sh
fi

# Start appropriate compose file
if [ "$GPU" = true ]; then
    echo "Starting with GPU support..."
    if [ "$DETACH" = true ]; then
        docker-compose -f docker-compose.gpu.yml up -d
    else
        docker-compose -f docker-compose.gpu.yml up
    fi
else
    echo "Starting with CPU..."
    if [ "$DETACH" = true ]; then
        docker-compose up -d
    else
        docker-compose up
    fi
fi