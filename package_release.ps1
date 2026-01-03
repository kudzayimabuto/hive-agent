# Package Release Script for Hive Agent
$ErrorActionPreference = "Stop"

Write-Host "Building Hive Agent (Release Mode)..."
Set-Location "hive-agent"
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed!"
}
Set-Location ..

$sourceDir = "target\release"
$destDir = "release_bundle"
$tokenizerPath = "hive-agent\tokenizer.json"

Write-Host "Creating release bundle at $destDir..."
if (Test-Path $destDir) {
    Remove-Item $destDir -Recurse -Force
}
New-Item -ItemType Directory -Path $destDir | Out-Null

Write-Host "Copying files..."
if (Test-Path "$sourceDir\hive-agent.exe") {
    Copy-Item "$sourceDir\hive-agent.exe" -Destination $destDir
} else {
    Write-Error "Could not find hive-agent.exe in $sourceDir. Build might have failed."
}

if (Test-Path $tokenizerPath) {
    Copy-Item $tokenizerPath -Destination $destDir
} else {
    Write-Warning "tokenizer.json not found in $tokenizerPath. Inference might fail."
}

# Check for CUDA
$hasCuda = $false
try {
    nvcc --version | Out-Null
    $hasCuda = $true
    Write-Host "CUDA Toolkit detected! Building GPU version..." -ForegroundColor Green
} catch {
    Write-Warning "CUDA Toolkit (nvcc) not found. Skipping GPU build."
}

if ($hasCuda) {
    Set-Location "hive-agent"
    cargo build --release --features cuda
    Set-Location ..
    Copy-Item "$sourceDir\hive-agent.exe" -Destination "$destDir\hive-agent-cuda.exe" -Force
}

$readmeContent = @"
HIVE AGENT - DRONE NODE
=======================

Instructions:
1. Ensure this laptop is connected to the same Wi-Fi/LAN as your main dashboard machine.
2. Double-click 'hive-agent.exe' to start the standard node.

GPU USERS (NVIDIA):
- If this machine has an NVIDIA GPU and CUDA drivers installed, run 'hive-agent-cuda.exe' instead for faster performance.

3. If Windows Firewall asks, allow access (Private Networks).
4. You should see "Swarm connected" in the terminal window.

Troubleshooting:
- If the window closes immediately, open a terminal here and run: .\hive-agent.exe
"@

Set-Content -Path "$destDir\README.txt" -Value $readmeContent

Write-Host "Done! Copy the 'release_bundle' folder to your second laptop."
