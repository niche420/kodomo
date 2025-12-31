# ML Optimizer for Game Streaming

Machine learning-based optimization for adaptive streaming quality and bitrate.

## Features

- **Bitrate Prediction**: LSTM network predicts optimal bitrate based on network conditions
- **Quality Optimization**: DQN agent optimizes resolution/FPS/quality tradeoffs
- **Real-time Adaptation**: Connects to streaming server via WebSocket
- **Online Learning**: Can train models in real-time (optional)

## Installation

```bash
cd python
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate
pip install -r requirements.txt
pip install -e .
```

## Usage

### Run Optimizer (Real-time)

```bash
python -m ml_optimizer.optimizer --server ws://localhost:8080/metrics
```

With training enabled:
```bash
python -m ml_optimizer.optimizer --server ws://localhost:8080/metrics --train
```

### Train Models (Offline)

```bash
python train.py --dataset data/metrics.json --epochs 200
```

### Evaluate Models

```bash
python evaluate.py --model models/bitrate_predictor.pt
```

## Architecture

### Bitrate Predictor (LSTM)
```
Input (10 features):
- Latency, jitter, packet loss
- Bandwidth, frame complexity
- Historical statistics

LSTM → FC Layers → Sigmoid → Bitrate (kbps)
```

### Quality Optimizer (DQN)
```
State (15 features):
- FPS, frame time, encode time
- Network metrics, buffer state
- Resource usage (CPU/GPU/Memory)

Q-Network → Action Selection → Configuration Changes
```

## Training Data Format

JSON format:
```json
[
  {
    "timestamp": 1234567890,
    "latency_ms": 25.3,
    "jitter_ms": 3.2,
    "packet_loss": 0.001,
    "bandwidth_kbps": 15000,
    "bitrate_kbps": 10000,
    "fps": 60,
    "quality_score": 0.92
  }
]
```

## Models Directory

After training, models saved to:
- `models/bitrate_predictor.pt` - Bitrate LSTM model
- `models/quality_optimizer.pt` - Quality DQN model

## Integration with Streaming Server

The optimizer connects to the streaming server via WebSocket and:
1. Receives real-time metrics
2. Predicts optimal configurations
3. Sends recommendations back to server

Server should implement endpoint: `ws://server:port/metrics`