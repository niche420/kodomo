set -e

echo "🚀 Deploying to Kubernetes"
echo ""

# Apply configurations
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/persistent-volumes.yaml
kubectl apply -f k8s/streaming-server.yaml
kubectl apply -f k8s/ml-optimizer.yaml
kubectl apply -f k8s/monitoring.yaml
kubectl apply -f k8s/hpa.yaml

# Wait for rollout
echo "Waiting for deployments..."
kubectl rollout status deployment/streaming-server -n game-streaming
kubectl rollout status deployment/ml-optimizer -n game-streaming
kubectl rollout status deployment/prometheus -n game-streaming
kubectl rollout status deployment/grafana -n game-streaming

echo ""
echo "✅ Deployment complete!"
echo ""
echo "Services:"
kubectl get svc -n game-streaming
echo ""
echo "Pods:"
kubectl get pods -n game-streaming