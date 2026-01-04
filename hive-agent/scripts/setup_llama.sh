#!/bin/bash
set -e

# Check and install essential build tools
echo "Checking dependencies..."
sudo apt-get update
sudo apt-get install -y build-essential cmake

# Check if we have an NVIDIA GPU
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected. Checking for CUDA toolkit..."
    if ! command -v nvcc &> /dev/null; then
        echo "CUDA Toolkit (nvcc) not found. Installing..."
        sudo apt-get install -y nvidia-cuda-toolkit
    fi
    
    # GCC 13/12 often causes issues with CUDA compilation in WSL
    # We force install GCC 11 for compatibility
    echo "Ensuring compatible GCC-11 is installed..."
    sudo apt-get install -y gcc-11 g++-11
    
    export CC=gcc-11
    export CXX=g++-11
    export CUDACXX=/usr/lib/nvidia-cuda-toolkit/bin/nvcc
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

# Detect GPU and Configure
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected. Building with CUDA support..."
    # User reported issues requiring explicit arch or fatbin off
    cmake .. \
        -DGGML_RPC=ON \
        -DGGML_CUDA=ON \
        -DGGML_CUDA_FATBIN=OFF \
        -DCMAKE_CUDA_ARCHITECTURES=native \
        -DCMAKE_BUILD_TYPE=Release
else
    echo "No NVIDIA GPU detected. Building for CPU..."
    cmake .. -DGGML_RPC=ON -DCMAKE_BUILD_TYPE=Release
fi

echo "Building..."
cmake --build . --config Release -j$(nproc)

echo "Build complete."
