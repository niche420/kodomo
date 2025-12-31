import torch
import torch.nn as nn
import numpy as np
from typing import List, Tuple
import logging

logger = logging.getLogger(__name__)


class BitratePredictor(nn.Module):
    """
    LSTM-based neural network to predict optimal bitrate.

    Inputs:
    - Network latency (RTT)
    - Jitter
    - Packet loss rate
    - Available bandwidth
    - Frame complexity
    - Historical bitrate stability

    Output:
    - Optimal bitrate (normalized 0-1, scaled to kbps)
    """

    def __init__(
            self,
            input_size: int = 10,
            hidden_size: int = 128,
            num_layers: int = 2,
            dropout: float = 0.2
    ):
        super().__init__()

        self.input_size = input_size
        self.hidden_size = hidden_size
        self.num_layers = num_layers

        # LSTM layers
        self.lstm = nn.LSTM(
            input_size=input_size,
            hidden_size=hidden_size,
            num_layers=num_layers,
            batch_first=True,
            dropout=dropout if num_layers > 1 else 0,
        )

        # Fully connected layers
        self.fc = nn.Sequential(
            nn.Linear(hidden_size, 64),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(64, 32),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(32, 1),
            nn.Sigmoid()  # Output 0-1
        )

        # Initialize weights
        self._init_weights()

    def _init_weights(self):
        """Initialize network weights"""
        for name, param in self.named_parameters():
            if 'weight' in name:
                nn.init.xavier_uniform_(param)
            elif 'bias' in name:
                nn.init.constant_(param, 0.0)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """
        Forward pass

        Args:
            x: (batch_size, sequence_length, input_size)

        Returns:
            predictions: (batch_size, 1)
        """
        # LSTM forward pass
        lstm_out, (h_n, c_n) = self.lstm(x)

        # Use last hidden state
        last_hidden = lstm_out[:, -1, :]

        # Fully connected layers
        output = self.fc(last_hidden)

        return output

    def predict_bitrate(
            self,
            features: np.ndarray,
            min_bitrate: int = 2000,
            max_bitrate: int = 20000
    ) -> int:
        """
        Predict optimal bitrate given network metrics

        Args:
            features: Network metrics (sequence_length, input_size)
            min_bitrate: Minimum bitrate in kbps
            max_bitrate: Maximum bitrate in kbps

        Returns:
            Predicted bitrate in kbps
        """
        self.eval()
        with torch.no_grad():
            # Add batch dimension
            x = torch.FloatTensor(features).unsqueeze(0)

            # Predict
            normalized = self.forward(x).item()

            # Scale to bitrate range
            bitrate = min_bitrate + (normalized * (max_bitrate - min_bitrate))

            return int(bitrate)


class NetworkMetrics:
    """Collect and process network metrics"""

    def __init__(self, window_size: int = 30):
        self.window_size = window_size

        # Metric histories
        self.latency_ms = []
        self.jitter_ms = []
        self.packet_loss = []
        self.bandwidth_kbps = []
        self.frame_complexity = []
        self.bitrate_kbps = []

        # Timestamps
        self.timestamps = []

    def add_sample(
            self,
            latency_ms: float,
            jitter_ms: float,
            packet_loss: float,
            bandwidth_kbps: float,
            frame_complexity: float = 0.5,
            bitrate_kbps: float = 0.0
    ):
        """Add a new metrics sample"""
        import time

        self.latency_ms.append(latency_ms)
        self.jitter_ms.append(jitter_ms)
        self.packet_loss.append(packet_loss)
        self.bandwidth_kbps.append(bandwidth_kbps)
        self.frame_complexity.append(frame_complexity)
        self.bitrate_kbps.append(bitrate_kbps)
        self.timestamps.append(time.time())

        # Keep only recent history
        if len(self.latency_ms) > self.window_size:
            self.latency_ms.pop(0)
            self.jitter_ms.pop(0)
            self.packet_loss.pop(0)
            self.bandwidth_kbps.pop(0)
            self.frame_complexity.pop(0)
            self.bitrate_kbps.pop(0)
            self.timestamps.pop(0)

    def get_features(self) -> np.ndarray:
        """
        Extract feature vector for prediction

        Returns:
            features: (sequence_length, 10) numpy array
        """
        if not self.latency_ms:
            return np.zeros((1, 10))

        # Stack time series features
        sequence_length = len(self.latency_ms)
        features = np.zeros((sequence_length, 10))

        for i in range(sequence_length):
            features[i] = [
                self.latency_ms[i] / 200.0,  # Normalize by max expected
                self.jitter_ms[i] / 50.0,
                self.packet_loss[i],
                self.bandwidth_kbps[i] / 50000.0,
                self.frame_complexity[i],
                np.mean(self.latency_ms[:i+1]) / 200.0 if i > 0 else 0,
                np.std(self.latency_ms[:i+1]) / 50.0 if i > 1 else 0,
                np.mean(self.jitter_ms[:i+1]) / 50.0 if i > 0 else 0,
                np.mean(self.packet_loss[:i+1]) if i > 0 else 0,
                self.bitrate_kbps[i] / 20000.0 if self.bitrate_kbps[i] > 0 else 0.5,
                ]

        return features