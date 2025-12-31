# Kubernetes Deployment

## Prerequisites

- Kubernetes cluster (1.25+)
- kubectl configured
- Persistent storage class
- LoadBalancer support (for services)
- NVIDIA GPU Operator (for GPU nodes)

## Quick Deploy

```bash
cd k8s

# Deploy everything
kubectl apply -f .

# Or use script
./deploy.sh

# Check status
kubectl get all -n game-streaming
```

## GPU Support

### Install NVIDIA GPU Operator

```bash
helm repo add nvidia https://helm.ngc.nvidia.com/nvidia
helm install gpu-operator nvidia/gpu-operator \
  --namespace gpu-operator-resources \
  --create-namespace
```

### Label GPU nodes

```bash
kubectl label nodes <node-name> accelerator=nvidia-gpu
```

### Deploy GPU servers

```bash
kubectl apply -f streaming-server-gpu.yaml
```

## Scaling

### Manual

```bash
kubectl scale deployment streaming-server -n game-streaming --replicas=5
```

### Auto-scaling

HPA automatically scales based on CPU/memory:
- Min replicas: 2
- Max replicas: 10
- Target CPU: 70%

## Monitoring

### Access Grafana

```bash
kubectl port-forward svc/grafana 3000:3000 -n game-streaming
# Open http://localhost:3000
```

### View Logs

```bash
kubectl logs -f deployment/streaming-server -n game-streaming
```

## Updates

### Rolling Update

```bash
kubectl set image deployment/streaming-server \
  streaming-server=streaming-server:v2.0 \
  -n game-streaming
```

### Rollback

```bash
kubectl rollout undo deployment/streaming-server -n game-streaming
```

## Troubleshooting

### Pods not starting

```bash
kubectl describe pod <pod-name> -n game-streaming
kubectl logs <pod-name> -n game-streaming
```

### Storage issues

```bash
kubectl get pvc -n game-streaming
kubectl describe pvc ml-models-pvc -n game-streaming
```