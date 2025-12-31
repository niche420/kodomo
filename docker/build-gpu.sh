set -e

echo "🎮 Building GPU-enabled images"
echo ""

# Check for NVIDIA Docker runtime
if ! docker run --rm --gpus all nvidia/cuda:12.2.0-base-ubuntu22.04 nvidia-smi &> /dev/null; then
    echo "⚠ Warning: NVIDIA Docker runtime not detected"
    echo "Install with: distribution=$(. /etc/os-release;echo \$ID\$VERSION_ID)"
    echo "              curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -"
    echo "              curl -s -L https://nvidia.github.io/nvidia-docker/\$distribution/nvidia-docker.list | sudo tee /etc/apt/sources.list.d/nvidia-docker.list"
    echo "              sudo apt-get update && sudo apt-get install -y nvidia-docker2"
    exit 1
fi

cd ..
cargo build --release --workspace --features nvenc

docker build -f docker/Dockerfile.server-gpu -t streaming-server-gpu:latest .

echo "✅ GPU image built"
nvidia-smi