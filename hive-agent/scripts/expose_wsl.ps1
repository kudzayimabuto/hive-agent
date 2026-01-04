# Check for Administrator privileges
if (!([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Warning "This script requires Administrator privileges to configure networking."
    Write-Warning "Please right-click PowerShell and select 'Run as Administrator', then try again."
    exit 1
}

# 1. Get WSL IP Address
$wsl_ip = (wsl hostname -I).Trim().Split(" ")[0]
if ([string]::IsNullOrWhiteSpace($wsl_ip)) {
    Write-Error "Could not determine WSL IP address. Is WSL running?"
    exit 1
}
Write-Host "Found WSL IP: $wsl_ip" -ForegroundColor Cyan

# 2. Configure Port Proxy (Forward Windows:50052 -> WSL:50052)
Write-Host "Configuring Port Proxy (50052)..." -ForegroundColor Yellow
netsh interface portproxy add v4tov4 listenport=50052 listenaddress=0.0.0.0 connectport=50052 connectaddress=$wsl_ip

# 3. Configure Windows Firewall
Write-Host "Configuring Windows Firewall..." -ForegroundColor Yellow
Remove-NetFirewallRule -DisplayName "Hive Agent Worker (WSL)" -ErrorAction SilentlyContinue
New-NetFirewallRule -DisplayName "Hive Agent Worker (WSL)" -Direction Inbound -LocalPort 50052 -Protocol TCP -Action Allow

Write-Host "Success! Port 50052 is now exposed to the LAN and forwarded to WSL." -ForegroundColor Green
Write-Host "You can now connect to this machine's IP from Node A." -ForegroundColor Green
