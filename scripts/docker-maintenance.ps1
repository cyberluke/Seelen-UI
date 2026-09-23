#Requires -Version 5
# Docker/WSL2 maintenance routine, run on each Windows login.

$ErrorActionPreference = 'Continue'
$vhdx = "$env:LOCALAPPDATA\Docker\wsl\disk\docker_data.vhdx"

# 1. Start Docker Desktop (also brings up WSL2) and wait until it is ready.
docker desktop start
$deadline = (Get-Date).AddSeconds(120)
do {
    Start-Sleep -Milliseconds 500
    $info = docker info --format '{{.ServerVersion}}' 2>$null
} while (-not $info -and (Get-Date) -lt $deadline)

# 2. Prune all unused resources.
docker system prune -a --force

# 3. Stop Docker and WSL2 so the VHDX is released.
docker desktop stop
wsl --shutdown

# 4. Compact the data disk.
Optimize-VHD -Path $vhdx -Mode Full

# 5. Start WSL2 and Docker Desktop again.
wsl --version | Out-Null
docker desktop start
