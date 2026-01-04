#!/bin/bash
set -e

# Check for required tools
if ! command -v g++ &> /dev/null; then
    echo "Error: g++ (C++ compiler) not found."
    echo "Please install it in WSL using: sudo apt-get update && sudo apt-get install -y build-essential"
    exit 1
fi

if ! command -v cmake &> /dev/null; then
    echo "Error: cmake not found."
    echo "Please install it in WSL using: sudo apt-get update && sudo apt-get install -y cmake"
    exit 1
fi

LLAMA_DIR="$HOME/llama.cpp"
REPO_URL="https://github.com/ggerganov/llama.cpp"

if [ -d "$LLAMA_DIR" ]; then
    echo "Updating llama.cpp..."
    cd "$LLAMA_DIR"
    git pull
else
    echo "Cloning llama.cpp..."
    git clone "$REPO_URL" "$LLAMA_DIR"
    cd "$LLAMA_DIR"
fi

echo "Configuring CMake..."
# Create build directory
mkdir -p build
cd build

# Detect GPU
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected. Building with CUDA support..."
    cmake .. -DGGML_RPC=ON -DGGML_CUDA=ON
else
    echo "No NVIDIA GPU detected. Building for CPU..."
    cmake .. -DGGML_RPC=ON
fi

echo "Building..."
cmake --build . --config Release -j$(nproc)

echo "Build complete."
