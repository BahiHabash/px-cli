# px Installation Guide

This guide walks through installing `px`, adding it to your `PATH`, setting proxy credentials, and confirming that it works.

Download the release assets from:

```text
https://github.com/BahiHabash/px-cli/releases
```

Each person should install `px` on their own machine and set their own credentials.

---
---

## macOS Apple Silicon

Use this guide for M-series Macs: M1, M2, M3, M4, and newer.

### 1. Download

Download this file from the latest GitHub Release:

```text
px-macos-arm64.tar.gz
```

### 2. Extract

If the file is in `~/Downloads`, run:

```bash
cd ~/Downloads
mkdir -p px-macos-arm64
tar -xzf px-macos-arm64.tar.gz -C px-macos-arm64
cd px-macos-arm64
```

### 3. Allow the binary to run

macOS may block unsigned binaries downloaded from the internet. Run:

```bash
chmod +x ./px
xattr -d com.apple.quarantine ./px
./px --help
```

If `./px --help` prints the help text, the binary works.

### 4. Add `px` to PATH

Install it into your user-local binary folder:

```bash
mkdir -p ~/.local/bin
cp ./px ~/.local/bin/px
chmod +x ~/.local/bin/px
```

Add `~/.local/bin` to your shell PATH:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

Check that the terminal can find it:

```bash
which px
px --version
```

Expected result:

```text
/Users/<your-user>/.local/bin/px
px <version>
```

### 5. Initialize px

```bash
px init
```

This creates the local config directory and auto-discovers supported developer tools.

### 6. Set credentials

Run:

```bash
px credentials set
```

Enter your proxy username, password, host, and port when prompted.

You can also set everything in one command:

```bash
px credentials set --user YOUR_USERNAME --pass YOUR_PASSWORD --host YOUR_PROXY_HOST --port YOUR_PROXY_PORT
```

Example:

```bash
px credentials set --user alice --pass secret --host 127.0.0.1 --port 1080
```

### 7. Confirm the proxy URL

```bash
px credentials show
```

Expected shape:

```text
Credentials file: /Users/<your-user>/Library/Application Support/proxy-launcher/.env
Proxy URL: socks5://YOUR_USERNAME:<redacted>@YOUR_PROXY_HOST:YOUR_PROXY_PORT
Host source: .env PX_HOST
Port source: .env PX_PORT
```

### 8. Check registered tools

```bash
px check
```

If `px init` discovered tools, run one of them:

```bash
px run codex
```

or:

```bash
px run cursor-d
```

Use `px ps` to inspect running tools that can be registered:

```bash
px ps
```

---
---

## Linux x64

Use this guide for x86_64 Linux machines.

### 1. Download

Download this file from the latest GitHub Release:

```text
px-linux-x64.tar.gz
```

### 2. Extract

If the file is in `~/Downloads`, run:

```bash
cd ~/Downloads
mkdir -p px-linux-x64
tar -xzf px-linux-x64.tar.gz -C px-linux-x64
cd px-linux-x64
```

### 3. Test the binary

```bash
chmod +x ./px
./px --help
```

If `./px --help` prints the help text, the binary works.

### 4. Add `px` to PATH

Install it into your user-local binary folder:

```bash
mkdir -p ~/.local/bin
cp ./px ~/.local/bin/px
chmod +x ~/.local/bin/px
```

Add `~/.local/bin` to your PATH.

For Bash:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

For Zsh:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

Check that the terminal can find it:

```bash
which px
px --version
```

Expected result:

```text
/home/<your-user>/.local/bin/px
px <version>
```

### 5. Initialize px

```bash
px init
```

This creates the local config directory and auto-discovers supported developer tools.

### 6. Set credentials

Run:

```bash
px credentials set
```

Enter your proxy username, password, host, and port when prompted.

You can also set everything in one command:

```bash
px credentials set --user YOUR_USERNAME --pass YOUR_PASSWORD --host YOUR_PROXY_HOST --port YOUR_PROXY_PORT
```

Example:

```bash
px credentials set --user alice --pass secret --host 127.0.0.1 --port 1080
```

### 7. Confirm the proxy URL

```bash
px credentials show
```

Expected shape:

```text
Credentials file: /home/<your-user>/.config/proxy-launcher/.env
Proxy URL: socks5://YOUR_USERNAME:<redacted>@YOUR_PROXY_HOST:YOUR_PROXY_PORT
Host source: .env PX_HOST
Port source: .env PX_PORT
```

### 8. Check registered tools

```bash
px check
```

If `px init` discovered tools, run one of them:

```bash
px run codex
```

or:

```bash
px run cursor-d
```

Use `px ps` to inspect running tools that can be registered:

```bash
px ps
```

---
---

## Windows x64

Use this guide for 64-bit Windows machines.

### 1. Download

Download this file from the latest GitHub Release:

```text
px-windows-x64.zip
```

### 2. Extract

Extract the zip file. It contains:

```text
px.exe
README.md
RELEASE_NOTES.md
```

For example, extract it to:

```text
C:\Users\<your-user>\Downloads\px-windows-x64
```

### 3. Test the binary

Open PowerShell in the extracted folder and run:

```powershell
.\px.exe --help
```

If `.\px.exe --help` prints the help text, the binary works.

### 4. Add `px` to PATH

Create a user-local binary folder:

```powershell
New-Item -ItemType Directory -Force "$HOME\.local\bin"
Copy-Item .\px.exe "$HOME\.local\bin\px.exe" -Force
```

Add that folder to your user PATH:

```powershell
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$InstallDir = "$HOME\.local\bin"
if (($UserPath -split ";") -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}
```

Close PowerShell and open a new PowerShell window.

Check that the terminal can find it:

```powershell
where.exe px
px --version
```

Expected result:

```text
C:\Users\<your-user>\.local\bin\px.exe
px <version>
```

### 5. Initialize px

```powershell
px init
```

This creates the local config directory and auto-discovers supported developer tools.

### 6. Set credentials

Run:

```powershell
px credentials set
```

Enter your proxy username, password, host, and port when prompted.

You can also set everything in one command:

```powershell
px credentials set --user YOUR_USERNAME --pass YOUR_PASSWORD --host YOUR_PROXY_HOST --port YOUR_PROXY_PORT
```

Example:

```powershell
px credentials set --user alice --pass secret --host 127.0.0.1 --port 1080
```

### 7. Confirm the proxy URL

```powershell
px credentials show
```

Expected shape:

```text
Credentials file: C:\Users\<your-user>\AppData\Roaming\proxy-launcher\.env
Proxy URL: socks5://YOUR_USERNAME:<redacted>@YOUR_PROXY_HOST:YOUR_PROXY_PORT
Host source: .env PX_HOST
Port source: .env PX_PORT
```

### 8. Check registered tools

```powershell
px check
```

If `px init` discovered tools, run one of them:

```powershell
px run codex
```

or:

```powershell
px run cursor-d
```

Use `px ps` to inspect running tools that can be registered:

```powershell
px ps
```

## Optional Script Install

If your machine allows direct script downloads, you can use the installer scripts instead of manually downloading archives.

macOS or Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/BahiHabash/px-cli/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/BahiHabash/px-cli/main/scripts/install.ps1 -UseB | iex
```

After script installation, still run:

```bash
px init
px credentials set
px credentials show
px check
```

On Windows, use the same commands in PowerShell.

## Success Checklist

The install is successful when all of these work:

```bash
px --version
px --help
px credentials show
px check
```

You should see a SOCKS5 proxy URL like:

```text
socks5://YOUR_USERNAME:<redacted>@YOUR_PROXY_HOST:YOUR_PROXY_PORT
```

Then run a registered app:

```bash
px run <shortcut>
```

Common shortcuts after `px init` may include:

```text
cursor-d
cursor
codex
code
kiro
claude
antigravity
```
