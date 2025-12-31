import torch
import torch.nn as nn
import numpy as np
from collections import deque
from typing import Dict, List, Tuple
import logging

logger = logging.getLogger(__name__)


class QualityOptimizer:
    """
    Deep Q-Network (DQN) for quality/performance optimization.

    Actions:
    0: Increase resolution
    1: Decrease resolution
    2: Increase FPS
    3: Decrease FPS
    4: Increase bitrate
    5: Decrease bitrate
    6: Use faster encoder preset
    7: Use slower encoder preset
    8: No change
    """

    STATE_SIZE = 15
    ACTION_SIZE = 9

    def __init__(
            self,
            gamma: float = 0.95,
            epsilon: float = 1.0,
            epsilon_min: float = 0.01,
            epsilon_decay: float = 0.995,
            learning_rate: float = 0.001,
            memory_size: int = 2000
    ):
        self.gamma = gamma
        self.epsilon = epsilon
        self.epsilon_min = epsilon_min
        self.epsilon_decay = epsilon_decay
        self.learning_rate = learning_rate

        # Experience replay memory
        self.memory = deque(maxlen=memory_size)

        # Q-networks
        self.model = self._build_model()
        self.target_model = self._build_model()
        self.update_target_model()

        # Optimizer
        self.optimizer = torch.optim.Adam(
            self.model.parameters(),
            lr=learning_rate
        )
        self.loss_fn = nn.MSELoss()

    def _build_model(self) -> nn.Module:
        """Build Q-network"""
        model = nn.Sequential(
            nn.Linear(self.STATE_SIZE, 128),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(128, 128),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(128, 64),
            nn.ReLU(),
            nn.Linear(64, self.ACTION_SIZE)
        )
        return model

    def update_target_model(self):
        """Copy weights from model to target model"""
        self.target_model.load_state_dict(self.model.state_dict())

    def get_state(self, metrics: Dict) -> np.ndarray:
        """
        Convert metrics to state vector

        Args:
            metrics: Dictionary of current metrics

        Returns:
            state: numpy array of shape (STATE_SIZE,)
        """
        state = np.array([
            metrics.get('fps', 60) / 120.0,
            metrics.get('frame_time_ms', 16) / 50.0,
            metrics.get('encode_time_ms', 5) / 20.0,
            metrics.get('latency_ms', 30) / 200.0,
            metrics.get('bitrate_kbps', 10000) / 30000.0,
            metrics.get('packet_loss', 0.0),
            metrics.get('buffer_occupancy', 0.5),
            metrics.get('cpu_usage', 0.5),
            metrics.get('gpu_usage', 0.5),
            metrics.get('memory_usage', 0.5),
            metrics.get('quality_score', 0.9),
            metrics.get('frame_drops', 0) / 10.0,
            metrics.get('jitter_ms', 5) / 30.0,
            metrics.get('bandwidth_kbps', 15000) / 30000.0,
            metrics.get('complexity_score', 0.5)
        ], dtype=np.float32)

        return state

    def act(self, state: np.ndarray) -> int:
        """
        Choose action using epsilon-greedy policy

        Args:
            state: Current state vector

        Returns:
            action: Action index (0-8)
        """
        # Exploration
        if np.random.rand() <= self.epsilon:
            return np.random.randint(self.ACTION_SIZE)

        # Exploitation
        self.model.eval()
        with torch.no_grad():
            state_tensor = torch.FloatTensor(state).unsqueeze(0)
            q_values = self.model(state_tensor)
            return torch.argmax(q_values).item()

    def remember(
            self,
            state: np.ndarray,
            action: int,
            reward: float,
            next_state: np.ndarray,
            done: bool
    ):
        """Store experience in replay memory"""
        self.memory.append((state, action, reward, next_state, done))

    def replay(self, batch_size: int = 32):
        """
        Train on batch from replay memory

        Args:
            batch_size: Number of samples to train on
        """
        if len(self.memory) < batch_size:
            return

        # Sample random batch
        indices = np.random.choice(len(self.memory), batch_size, replace=False)
        batch = [self.memory[i] for i in indices]

        # Unpack batch
        states = torch.FloatTensor([s for s, _, _, _, _ in batch])
        actions = torch.LongTensor([a for _, a, _, _, _ in batch])
        rewards = torch.FloatTensor([r for _, _, r, _, _ in batch])
        next_states = torch.FloatTensor([ns for _, _, _, ns, _ in batch])
        dones = torch.FloatTensor([d for _, _, _, _, d in batch])

        # Current Q values
        self.model.train()
        current_q = self.model(states).gather(1, actions.unsqueeze(1))

        # Target Q values
        with torch.no_grad():
            next_q = self.target_model(next_states).max(1)[0]
            target_q = rewards + (1 - dones) * self.gamma * next_q

        # Loss and backprop
        loss = self.loss_fn(current_q.squeeze(), target_q)

        self.optimizer.zero_grad()
        loss.backward()
        self.optimizer.step()

        # Decay epsilon
        if self.epsilon > self.epsilon_min:
            self.epsilon *= self.epsilon_decay

        return loss.item()

    def calculate_reward(
            self,
            metrics: Dict,
            prev_metrics: Dict
    ) -> float:
        """
        Calculate reward based on metrics

        Reward components:
        + High quality score
        + Stable FPS
        + Low latency
        - High frame drops
        - High CPU usage
        """
        reward = 0.0

        # Quality (most important)
        quality = metrics.get('quality_score', 0.9)
        reward += quality * 10.0

        # FPS stability
        fps = metrics.get('fps', 60)
        target_fps = 60
        fps_penalty = abs(fps - target_fps) / target_fps
        reward -= fps_penalty * 5.0

        # Latency penalty
        latency = metrics.get('latency_ms', 30)
        if latency > 50:
            reward -= (latency - 50) * 0.1

        # Frame drops penalty
        frame_drops = metrics.get('frame_drops', 0)
        reward -= frame_drops * 2.0

        # CPU efficiency bonus
        cpu = metrics.get('cpu_usage', 0.5)
        if cpu < 0.6:
            reward += (0.6 - cpu) * 2.0

        # Packet loss penalty
        packet_loss = metrics.get('packet_loss', 0.0)
        reward -= packet_loss * 20.0

        return reward

    def save(self, path: str):
        """Save model weights"""
        torch.save({
            'model_state_dict': self.model.state_dict(),
            'target_model_state_dict': self.target_model.state_dict(),
            'optimizer_state_dict': self.optimizer.state_dict(),
            'epsilon': self.epsilon,
        }, path)
        logger.info(f"Model saved to {path}")

    def load(self, path: str):
        """Load model weights"""
        checkpoint = torch.load(path)
        self.model.load_state_dict(checkpoint['model_state_dict'])
        self.target_model.load_state_dict(checkpoint['target_model_state_dict'])
        self.optimizer.load_state_dict(checkpoint['optimizer_state_dict'])
        self.epsilon = checkpoint['epsilon']
        logger.info(f"Model loaded from {path}")