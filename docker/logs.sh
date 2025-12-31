SERVICE=${1:-streaming-server}

echo "📋 Showing logs for: $SERVICE"
echo "Press Ctrl+C to exit"
echo ""

docker-compose logs -f $SERVICE