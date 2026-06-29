$ErrorActionPreference = "Stop"

$Repo = if ($env:PX_REPO) { $env:PX_REPO } else { "BahiHabash/px-cli" }
$Tag = if ($env:PX_VERSION) { $env:PX_VERSION } else { "v2" }
$InstallDir = if ($env:PX_INSTALL_DIR) { $env:PX_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
$Asset = "px-windows.exe"
$Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"
$Destination = Join-Path $InstallDir "px.exe"
$Temp = Join-Path ([System.IO.Path]::GetTempPath()) "px-install.exe"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "Downloading $Url"
Invoke-WebRequest -Uri $Url -OutFile $Temp
Move-Item -Force $Temp $Destination

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$PathParts = @()
if ($UserPath) {
    $PathParts = $UserPath -split ";"
}

if ($PathParts -notcontains $InstallDir) {
    $NextPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable("Path", $NextPath, "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "Added $InstallDir to your user PATH. Restart your terminal if px is not found."
}

Write-Host "Installed px to $Destination"
& $Destination --version
