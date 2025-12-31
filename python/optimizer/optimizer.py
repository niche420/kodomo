import asyncio
import websockets
import json
import logging
import signal
import sys
from pathlib import Path
from typing import Dict, Optional

import torch
import numpy as np

from .bitrate_predictor import BitratePredictor, NetworkMetrics
from .quality_optimizer import QualityOptimizer
from .data_collector import MetricsCollector

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class StreamingOptimizer:
    """
    Main optimizer that connects to streaming server and provides
    real-time ML-based optimization recommendations.
    """

    def __init__(
            self,
            server_url: str = "ws://localhost:8080/metrics",
            models_dir: str = "models",
            enable_training: bool = False
    ):
        self.server_url = server_url
        self.models_dir = Path(models_dir)
        self.enable_training = enable_training

        # Initialize models
        self.bitrate_model = BitratePredictor()
        self.quality_model = QualityOptimizer()
        self.network_metrics = NetworkMetrics(window_size=30)

        # Load pre-trained models if available
        self._load_models()

        # Metrics collector
        self.collector = MetricsCollector(server_url)

        # State tracking
        self.current_metrics = {}
        self.previous_metrics = {}
        self.recommendations_sent = 0
        self.running = False

    def _load_models(self):
        """Load pre-trained models"""
        bitrate_path = self.models_dir / "bitrate_predictor.pt"
        quality_path = self.models_dir / "quality_optimizer.pt"

        if bitrate_path.exists():
            try:
                self.bitrate_model.load_state_dict(
                    torch.load(bitrate_path, map_location='cpu')
                )
                logger.info(f"Loaded bitrate model from {bitrate_path}")
            except Exception as e:
                logger.warning(f"Failed to load bitrate model: {e}")

        if quality_path.exists():
            try:
                self.quality_model.load(str(quality_path))
                logger.info(f"Loaded quality model from {quality_path}")
            except Exception as e:
                logger.warning(f"Failed to load quality model: {e}")

    def _save_models(self):
        """Save trained models"""
        self.models_dir.mkdir(exist_ok=True)

        bitrate_path = self.models_dir / "bitrate_predictor.pt"
        quality_path = self.models_dir / "quality_optimizer.pt"

        torch.save(self.bitrate_model.state_dict(), bitrate_path)
        self.quality_model.save(str(quality_path))

        logger.info("Models saved")

    async def optimize_bitrate(self) -> int:
        """
        Predict optimal bitrate using LSTM model

        Returns:
            Recommended bitrate in kbps
        """
        # Get feature vector from network metrics
        features = self.network_metrics.get_features()

        if len(features) < 5:
            # Not enough data yet, use current bitrate
            return self.current_metrics.get('bitrate_kbps', 10000)

        # Predict optimal bitrate
        optimal_bitrate = self.bitrate_model.predict_bitrate(
            features,
            min_bitrate=2000,
            max_bitrate=20000
        )

        logger.debug(f"Predicted bitrate: {optimal_bitrate} kbps")
        return optimal_bitrate

    def optimize_quality(self) -> Dict:
        """
        Use DQN to determine quality adjustments

        Returns:
            Dictionary with recommended actions
        """
        # Get current state
        state = self.quality_model.get_state(self.current_metrics)

        # Choose action
        action = self.quality_model.act(state)

        # Map action to recommendation
        recommendations = self._action_to_recommendation(action)

        # If training enabled, store experience
        if self.enable_training and self.previous_metrics:
            prev_state = self.quality_model.get_state(self.previous_metrics)
            reward = self.quality_model.calculate_reward(
                self.current_metrics,
                self.previous_metrics
            )

            self.quality_model.remember(
                prev_state,
                action,
                reward,
                state,
                False  # not done
            )

            # Train on batch periodically
            if self.recommendations_sent % 10 == 0:
                loss = self.quality_model.replay(batch_size=32)
                if loss is not None:
                    logger.debug(f"Training loss: {loss:.4f}")

        return recommendations

    def _action_to_recommendation(self, action: int) -> Dict:
        """Convert action index to recommendation"""
        current_res = self.current_metrics.get('resolution', '1920x1080')
        current_fps = self.current_metrics.get('fps', 60)
        current_bitrate = self.current_metrics.get('bitrate_kbps', 10000)

        recommendation = {
            'action': 'no_change',
            'reason': ''
        }

        if action == 0:  # Increase resolution
            recommendation['action'] = 'increase_resolution'
            recommendation['reason'] = 'Network stable, can increase quality'
        elif action == 1:  # Decrease resolution
            recommendation['action'] = 'decrease_resolution'
            recommendation['reason'] = 'Network congested, reduce resolution'
        elif action == 2:  # Increase FPS
            recommendation['action'] = 'increase_fps'
            recommendation['target_fps'] = min(current_fps + 10, 120)
        elif action == 3:  # Decrease FPS
            recommendation['action'] = 'decrease_fps'
            recommendation['target_fps'] = max(current_fps - 10, 30)
        elif action == 4:  # Increase bitrate
            recommendation['action'] = 'increase_bitrate'
            recommendation['target_bitrate'] = min(current_bitrate + 1000, 20000)
        elif action == 5:  # Decrease bitrate
            recommendation['action'] = 'decrease_bitrate'
            recommendation['target_bitrate'] = max(current_bitrate - 1000, 2000)
        elif action == 6:  # Faster preset
            recommendation['action'] = 'use_faster_preset'
            recommendation['reason'] = 'CPU overloaded'
        elif action == 7:  # Slower preset
            recommendation['action'] = 'use_slower_preset'
            recommendation['reason'] = 'CPU underutilized, can improve quality'
        else:  # No change
            recommendation['action'] = 'no_change'

        return recommendation

    async def run(self):
        """Main optimization loop"""
        logger.info(f"🤖 Starting ML Optimizer")
        logger.info(f"   Server: {self.server_url}")
        logger.info(f"   Training: {'Enabled' if self.enable_training else 'Disabled'}")
        logger.info("")

        self.running = True

        try:
            # Connect to metrics stream
            await self.collector.connect()

            while self.running:
                try:
                    # Receive metrics from server
                    metrics = await asyncio.wait_for(
                        self.collector.collect(),
                        timeout=5.0
                    )

                    if not metrics:
                        continue

                    self.previous_metrics = self.current_metrics.copy()
                    self.current_metrics = metrics

                    # Update network metrics
                    self.network_metrics.add_sample(
                        latency_ms=metrics.get('latency_ms', 30),
                        jitter_ms=metrics.get('jitter_ms', 5),
                        packet_loss=metrics.get('packet_loss', 0.0),
                        bandwidth_kbps=metrics.get('bandwidth_kbps', 15000),
                        frame_complexity=metrics.get('complexity', 0.5),
                        bitrate_kbps=metrics.get('bitrate_kbps', 10000)
                    )

                    # Get recommendations
                    optimal_bitrate = await self.optimize_bitrate()
                    quality_rec = self.optimize_quality()

                    # Combine recommendations
                    recommendations = {
                        'bitrate_kbps': optimal_bitrate,
                        'quality_action': quality_rec,
                        'confidence': float(1.0 - self.quality_model.epsilon),
                        'timestamp': metrics.get('timestamp', 0)
                    }

                    # Send back to server
                    # TODO: Implement WebSocket send to server

                    self.recommendations_sent += 1

                    if self.recommendations_sent % 10 == 0:
                        logger.info(
                            f"📊 Recommendations: bitrate={optimal_bitrate} kbps, "
                            f"action={quality_rec['action']}, "
                            f"sent={self.recommendations_sent}"
                        )

                except asyncio.TimeoutError:
                    logger.warning("Timeout waiting for metrics")
                except Exception as e:
                    logger.error(f"Error in optimization loop: {e}")
                    await asyncio.sleep(1)

        finally:
            await self.collector.close()

            if self.enable_training:
                self._save_models()

            logger.info("ML Optimizer stopped")

    def stop(self):
        """Stop the optimizer"""
        self.running = False


# Signal handler for graceful shutdown
def signal_handler(sig, frame):
    logger.info("Received interrupt signal")
    sys.exit(0)


async def main():
    """Main entry point"""
    import argparse

    parser = argparse.ArgumentParser(description='ML Streaming Optimizer')
    parser.add_argument(
        '--server',
        default='ws://localhost:8080/metrics',
        help='WebSocket URL for metrics stream'
    )
    parser.add_argument(
        '--models-dir',
        default='models',
        help='Directory for model storage'
    )
    parser.add_argument(
        '--train',
        action='store_true',
        help='Enable online training'
    )
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Enable debug logging'
    )

    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    # Setup signal handler
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # Create and run optimizer
    optimizer = StreamingOptimizer(
        server_url=args.server,
        models_dir=args.models_dir,
        enable_training=args.train
    )

    try:
        await optimizer.run()
    except KeyboardInterrupt:
        logger.info("Interrupted by user")
        optimizer.stop()


if __name__ == '__main__':
    asyncio.run(main())