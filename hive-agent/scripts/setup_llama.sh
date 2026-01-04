#!/bin/bash
set -e

LLAMA_DIR="$HOME/llama.cpp"
REPO_URL="https://github.com/ggerganov/llama.cpp"

# 1. Clone or Update
if [ -d "$LLAMA_DIR" ]; then
    echo "Updating llama.cpp..."
    cd "$LLAMA_DIR"
    git pull
else
    echo "Cloning llama.cpp..."
    git clone "$REPO_URL" "$LLAMA_DIR"
    cd "$LLAMA_DIR"
fi

# 2. Detect GPU (NVIDIA)
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected. Building with CUDA support..."
    BUILD_FLAGS="LLAMA_CUDA=1 LLAMA_RPC=1"
else
    echo "No NVIDIA GPU detected. Building for CPU..."
    BUILD_FLAGS="LLAMA_RPC=1"
fi

# 3. Build
echo "Building with flags: $BUILD_FLAGS"
make $BUILD_FLAGS -j$(nproc)

echo "Build complete."
