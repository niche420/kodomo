SERVICE=${1:-streaming-server}

echo "🐚 Opening shell in: $SERVICE"

docker-compose exec $SERVICE /bin/bash