# Docker Deployment Guide

## Quick Start

```bash
cd docker

# Build images
./build.sh

# Start services
./run.sh

# Or with GPU support
./run.sh --gpu

# Stop services
./stop.sh
```

## Services

### Streaming Server
- **Image**: `streaming-server:latest`
- **Ports**: 8080/udp (streaming), 9090 (metrics)
- **Config**: `config.toml`

### ML Optimizer
- **Image**: `ml-optimizer:latest`
- **Depends on**: streaming-server
- **Volumes**: ml-models, ml-data

### Prometheus
- **Image**: `prom/prometheus:latest`
- **Port**: 9091
- **Config**: `prometheus.yml`

### Grafana
- **Image**: `grafana/grafana:latest`
- **Port**: 3000
- **Default credentials**: admin/admin

## GPU Support

### Requirements
- NVIDIA GPU
- NVIDIA drivers installed
- nvidia-docker2 runtime

### Install NVIDIA Docker Runtime

```bash
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
    sudo tee /etc/apt/sources.list.d/nvidia-docker.list

sudo apt-get update
sudo apt-get install -y nvidia-docker2
sudo systemctl restart docker
```

### Test GPU Access

```bash
docker run --rm --gpus all nvidia/cuda:12.2.0-base-ubuntu22.04 nvidia-smi
```

### Run with GPU

```bash
./build-gpu.sh
./run.sh --gpu
```

## Configuration

### Server Config

Edit `docker/config.toml`:

```toml
[video]
width = 1920
height = 1080
fps = 60
bitrate_kbps = 10000
hw_accel = false  # true for GPU
```

### Environment Variables

```bash
# In docker-compose.yml
environment:
  - RUST_LOG=debug  # Logging level
  - RUST_BACKTRACE=1  # Enable backtraces
```

## Volumes

### Persistent Data

- `ml-models`: Trained ML models
- `ml-data`: Training data
- `prometheus-data`: Metrics history
- `grafana-data`: Dashboards and settings

### Backup Volumes

```bash
docker run --rm -v docker_ml-models:/data -v $(pwd):/backup \
    ubuntu tar czf /backup/ml-models-backup.tar.gz /data
```

### Restore Volumes

```bash
docker run --rm -v docker_ml-models:/data -v $(pwd):/backup \
    ubuntu tar xzf /backup/ml-models-backup.tar.gz -C /
```

## Networking

### Bridge Network

Default network: `streaming-net`

Services communicate via service names:
- `streaming-server:8080`
- `ml-optimizer:8081`
- `prometheus:9090`

### Port Mapping

| Service | Internal | External |
|---------|----------|----------|
| Server (UDP) | 8080 | 8080 |
| Server (Metrics) | 9090 | 9090 |
| Prometheus | 9090 | 9091 |
| Grafana | 3000 | 3000 |

## Monitoring

### Grafana Dashboards

Access: http://localhost:3000

Dashboards:
- Streaming Performance
- Network Metrics
- ML Optimizer Stats

### Prometheus Metrics

Access: http://localhost:9091

Query examples:
```
streaming_fps
streaming_bitrate_kbps
streaming_latency_ms
ml_predictions_total
```

## Troubleshooting

### Container won't start

```bash
# Check logs
docker-compose logs streaming-server

# Check health
docker-compose ps
```

### Server not capturing screen

In Docker, screen capture may not work. Use:
- VNC/X11 forwarding
- Run server on host, only client in Docker

### GPU not detected

```bash
# Verify GPU accessible
docker run --rm --gpus all nvidia/cuda:12.2.0-base nvidia-smi

# Check nvidia-docker runtime
docker info | grep -i runtime
```

### Out of memory

Increase Docker memory limit:
```bash
# Docker Desktop: Settings > Resources > Memory
# Or edit daemon.json
{
  "default-runtime": "nvidia",
  "runtimes": {
    "nvidia": {
      "path": "nvidia-container-runtime",
      "args": []
    }
  }
}
```

## Production Deployment

### Security

1. Change default passwords
2. Enable TLS/SSL
3. Use secrets management
4. Restrict network access

### Performance

1. Use GPU for encoding
2. Tune network buffer sizes
3. Enable Prometheus metrics
4. Set resource limits

### High Availability

Use Docker Swarm or Kubernetes for:
- Load balancing
- Auto-scaling
- Health checks
- Rolling updates

## Commands Reference

```bash
# Build
./build.sh           # CPU images
./build-gpu.sh       # GPU images

# Run
./run.sh             # Start CPU
./run.sh --gpu       # Start GPU
./run.sh -d          # Detached mode

# Manage
./stop.sh            # Stop all
./logs.sh <service>  # View logs
./shell.sh <service> # Open shell
./clean.sh           # Remove all

# Test
./test.sh            # Run tests
```