$ErrorActionPreference = "Stop"

$Repo = if ($env:PX_REPO) { $env:PX_REPO } else { "BahiHabash/px-cli" }
$Tag = if ($env:PX_VERSION) { $env:PX_VERSION } else { "v2.0.2" }
$InstallDir = if ($env:PX_INSTALL_DIR) { $env:PX_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
$Asset = "px-windows-x64.zip"
$Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"
$Destination = Join-Path $InstallDir "px.exe"
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("px-install-" + [System.Guid]::NewGuid().ToString("N"))
$Archive = Join-Path $TempDir $Asset
$ExtractDir = Join-Path $TempDir "package"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

Write-Host "Downloading $Url"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try {
    Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing
} catch {
    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
        & curl.exe -fL $Url -o $Archive
    } else {
        throw
    }
}

Expand-Archive -Path $Archive -DestinationPath $ExtractDir -Force
Move-Item -Force (Join-Path $ExtractDir "px.exe") $Destination
Remove-Item -Recurse -Force $TempDir

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
