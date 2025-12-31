"""
Offline training script for ML models
"""
import argparse
import logging
from pathlib import Path

from optimizer.trainer import Trainer

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def main():
    parser = argparse.ArgumentParser(description='Train ML models')
    parser.add_argument(
        '--dataset',
        required=True,
        help='Path to training dataset'
    )
    parser.add_argument(
        '--models-dir',
        default='models',
        help='Directory to save models'
    )
    parser.add_argument(
        '--epochs',
        type=int,
        default=100,
        help='Number of training epochs'
    )
    parser.add_argument(
        '--batch-size',
        type=int,
        default=32,
        help='Training batch size'
    )
    parser.add_argument(
        '--learning-rate',
        type=float,
        default=0.001,
        help='Learning rate'
    )

    args = parser.parse_args()

    logger.info("🧠 Starting model training")
    logger.info(f"   Dataset: {args.dataset}")
    logger.info(f"   Models dir: {args.models_dir}")
    logger.info(f"   Epochs: {args.epochs}")
    logger.info("")

    # Create trainer
    trainer = Trainer(models_dir=args.models_dir)

    # Train bitrate predictor
    trainer.train_bitrate_predictor(
        dataset_path=args.dataset,
        epochs=args.epochs,
        batch_size=args.batch_size,
        learning_rate=args.learning_rate
    )

    logger.info("✅ Training complete!")


if __name__ == '__main__':
    main()