import json
import asyncio
import websockets
from typing import Dict, List
import logging

logger = logging.getLogger(__name__)


class MetricsCollector:
    """Collect metrics from streaming server"""

    def __init__(self, server_url: str = "ws://localhost:8080/metrics"):
        self.server_url = server_url
        self.metrics_history = []

    async def connect(self):
        """Connect to metrics stream"""
        self.websocket = await websockets.connect(self.server_url)
        logger.info(f"Connected to metrics stream: {self.server_url}")

    async def collect(self) -> Dict:
        """Receive metrics from server"""
        try:
            message = await self.websocket.recv()
            metrics = json.loads(message)
            self.metrics_history.append(metrics)
            return metrics
        except Exception as e:
            logger.error(f"Error collecting metrics: {e}")
            return {}

    async def close(self):
        """Close connection"""
        if hasattr(self, 'websocket'):
            await self.websocket.close()

    def get_history(self, window: int = 100) -> List[Dict]:
        """Get recent metrics history"""
        return self.metrics_history[-window:]

    def save_history(self, path: str):
        """Save metrics history to file"""
        with open(path, 'w') as f:
            json.dump(self.metrics_history, f, indent=2)
        logger.info(f"Saved {len(self.metrics_history)} metrics to {path}")