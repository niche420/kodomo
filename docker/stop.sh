echo "🛑 Stopping streaming services..."

docker-compose down
docker-compose -f docker-compose.gpu.yml down 2>/dev/null || true

echo "✅ All services stopped"