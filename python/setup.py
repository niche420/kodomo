from setuptools import setup, find_packages

setup(
    name="kodomo-optimizer",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "torch>=2.1.0",
        "numpy>=1.24.0",
        "pandas>=2.0.0",
        "websockets>=11.0",
    ],
    python_requires=">=3.9",
)