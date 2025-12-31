echo "🧹 Cleaning Docker resources..."

# Stop containers
docker-compose down -v

# Remove images
docker rmi streaming-server:latest 2>/dev/null || true
docker rmi streaming-server-gpu:latest 2>/dev/null || true
docker rmi ml-optimizer:latest 2>/dev/null || true
docker rmi streaming-client:latest 2>/dev/null || true

# Remove volumes
docker volume rm docker_ml-models 2>/dev/null || true
docker volume rm docker_ml-data 2>/dev/null || true
docker volume rm docker_prometheus-data 2>/dev/null || true
docker volume rm docker_grafana-data 2>/dev/null || true

# Prune
docker system prune -f

echo "✅ Cleanup complete"