# px (Proxy Launcher)

`px` is a proxy-injector and application launcher. It intercepts your registered developer tools and seamlessly injects proxy configuration into their environment before launching them — via a **native shell wrapper** — making it easy to route AI/LLM traffic through a corporate or personal proxy without touching individual applications.

## Features

- **Auto-Discovery**: Scans your system for supported developer tools (VS Code, Cursor, Codex, Antigravity, Kiro, Claude, RustRover, Vim, etc.), resolves their executable paths (including Windows Store aliases), and automatically registers them.
- **Interactive Process Scanner**: When manually registering an app, `px` can scan your running processes and help you pick the exact executable path.
- **Shell-Wrapper Launcher**: Every app is launched via the platform's native shell (`/bin/sh` on Unix, `cmd` on Windows). CLI tools use `exec`; desktop apps use a detached launch so they survive terminal shutdown.
- **AI-Only Proxy Mode**: Route *only* AI/LLM API traffic through the proxy. Non-AI traffic (npm, git, pip, crates.io, GitHub, etc.) bypasses the proxy entirely via `NO_PROXY`. Auto-enabled for AI tools like Cursor, Codex, Antigravity, Kiro, and Claude.
- **Secure Credential Storage**: Stores proxy credentials locally in a `.env` file instead of committing them to version control.
- **Dynamic Config Overrides**: Reads `PX_HOST` and `PX_PORT` from your environment to dynamically override the proxy host/port.
- **App Execution Classes**: Supports both `cli` (blocking execution, inherits terminal IO) and `desktop` (detached execution, frees the terminal immediately).
- **Portable Configurations**: Saves paths as portable environment variables (e.g., `%LOCALAPPDATA%\...`) making the `config.toml` easily shareable.

## Setup & Installation

### Recommended Installer

macOS/Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/BahiHabash/px-cli/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/BahiHabash/px-cli/main/scripts/install.ps1 -UseB | iex
```

The installers download the right release asset, install it as `px`, and use `~/.local/bin` by default.

### Manual Install

1. **Install a release binary**: Download the binary for your platform from the GitHub release and place it somewhere in your `PATH`.
   - Linux: rename `px-linux` to `px`
   - macOS Apple Silicon: rename `px-mac-silicon` to `px`
   - macOS Intel: rename `px-mac-intel` to `px`
   - Windows: use `px-windows.exe`, or rename it to `px.exe`
2. **Or build locally**: Run `cargo build --release` and place the executable (`target/release/px` or `target/release/px.exe`) in your system's PATH.
3. **Initialize**: Run `px init`.
   - Creates the platform configuration directory (e.g., `~/.config/proxy-launcher/` or `%APPDATA%\proxy-launcher\`).
   - Scaffolds a default `config.toml` and a `.env` template.
   - Runs the auto-discovery engine to find and register any supported tools — AI tools are automatically set to AI-only proxy mode.
4. **Configure Credentials**: Run the credentials helper and follow the prompts:
   ```bash
   px credentials set
   ```
   Or configure everything non-interactively:
   ```bash
   px credentials set --user your_username --pass your_password --host 127.0.0.1 --port 8080
   ```

## Usage

### Run the Tool

```bash
px --help
px init
px credentials set
px check
px run <shortcut>
```

After `px init`, common auto-discovered shortcuts include `cursor-d`, `cursor`, `codex`, `code`, `kiro`, `claude`, and `antigravity`.

### Launching Apps
To launch a registered application with proxy variables injected:
```bash
px run <shortcut>
```
Example:
```bash
px run cursor-d   # desktop app; launches with [ai-only] mode
px run codex      # CLI tool; npm/git/pip bypass the proxy
```

You can also bypass the `.env` file for a specific run by providing a runtime override:
```bash
px run codex --proxy-override "socks5://user:pass@10.0.0.1:8080"
```

### Configuring Credentials

Credentials are stored in the generated `.env` file next to `config.toml`.
Use the helper command for normal setup:

```bash
px credentials set
```

To update one field without prompts:

```bash
px credentials set --host proxy.company.internal --port 8080
```

To inspect the resolved proxy URL with the password redacted:

```bash
px credentials show
```

## Scope & Limitations

`px` currently builds SOCKS5 proxy URLs and injects them through `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`. HTTP proxy URL generation is not the default path in this release.

The tool is mainly designed to handle AI work in developer tools: routing LLM/API traffic while keeping package managers, Git forges, local services, and normal development traffic direct where possible. It does not provide full IDE control, does not proxy every IDE action, and does not currently support MCP server routing or IDE extension marketplace workflows as first-class features.

Some external network actions may not work 100% correctly through this setup yet, especially remote workflows such as `git push`, Git remote operations, and other tool-specific background network calls.

### Managing Shortcuts

**Register a new app:**
```bash
px register --name my-app --path "C:\path\to\app.exe" --kind desktop
# With AI-only proxy mode:
px register --name cursor-d --path "/Applications/Cursor.app/Contents/MacOS/Cursor" --kind desktop --ai-only
```
*(If you omit `--path`, `px register` will launch the interactive process scanner to help you find the running app's executable).*

**Rename an existing shortcut:**
```bash
px alias <old-name> <new-name>
```

**Edit Configuration:**
To open `config.toml` in your system's default text editor:
```bash
px edit
```

**Check Configuration:**
Validate your `config.toml` and verify that all registered executable paths exist on your disk:
```bash
px check
```

## AI-Only Proxy Mode

When `ai_only_proxy = true` is set for an app, `px` sets both the proxy **and** a broad `NO_PROXY` exclusion list. This means:

```
ai.openai.com     → NOT in NO_PROXY → routes through proxy  ✓
api.anthropic.com → NOT in NO_PROXY → routes through proxy  ✓
api.cursor.sh     → NOT in NO_PROXY → routes through proxy  ✓
github.com        → IN NO_PROXY     → direct connection     ✓
registry.npmjs.org→ IN NO_PROXY     → direct connection     ✓
localhost:3000    → IN NO_PROXY     → direct connection     ✓
```

### Built-in `NO_PROXY` Baseline

The following are always excluded from the proxy in AI-only mode:

| Category              | Examples                                          |
|-----------------------|---------------------------------------------------|
| Loopback              | `localhost`, `127.0.0.1`, `::1`                   |
| Private IP ranges     | `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`  |
| npm                   | `registry.npmjs.org`, `npmjs.org`                 |
| Python                | `pypi.org`, `files.pythonhosted.org`              |
| Rust/Cargo            | `crates.io`, `static.crates.io`                   |
| Go modules            | `proxy.golang.org`, `sum.golang.org`              |
| Git forges            | `github.com`, `gitlab.com`, `bitbucket.org`       |
| Container registries  | `docker.io`, `ghcr.io`, `gcr.io`                  |

### Adding Corporate Hosts

Add your internal hosts to `no_proxy_extra` in `config.toml`:

```toml
[proxy]
host           = "127.0.0.1"
port           = 8080
cert_path      = ""
no_proxy_extra = ["registry.company.internal", "git.company.internal"]
```

## How It Works

### Shell-Wrapper Launch

Instead of spawning the app directly, `px` always launches via the platform shell:

| Platform | Shell     | Command                                          |
|----------|-----------|--------------------------------------------------|
| Unix CLI | `/bin/sh` | `sh -c 'HTTP_PROXY="..." exec /path/to/tool'`   |
| Unix desktop | `/bin/sh` | `sh -c 'HTTP_PROXY="..." nohup /path/to/app >/dev/null 2>&1 &'` |
| Windows  | `cmd`     | `cmd /c "set HTTP_PROXY=...&& path\to\app.exe"` |

On Unix, CLI launches use `exec` to replace the shell process. Desktop launches use `nohup` in the background, so closing the terminal does not close the app.

### Full Flow

1. `px` reads the `.env` file to retrieve `PX_PROXY_USER`, `PX_PROXY_PASS`, `PX_HOST`, and `PX_PORT`.
2. It constructs a SOCKS5 proxy URL (e.g., `socks5://user:pass@127.0.0.1:8080`).
3. If `ai_only_proxy = true`, it also builds the `NO_PROXY` exclusion list.
4. It resolves the shortcut name against `config.toml` to find the exact path and arguments.
5. It builds a shell command string with all env vars exported inline.
6. It spawns the shell. Depending on the `kind` (`cli` vs `desktop`), it either blocks or detaches.

## Release Notes

### v2

- Adds SOCKS5-first proxy URL generation for credential-based runs.
- Adds `px credentials set` and `px credentials show` for safer local `.env` management.
- Adds shorter built-in shortcuts such as `cursor-d`, `codex`, `kiro`, `claude`, and `antigravity`.
- Expands auto-discovery for AI-focused developer tools and keeps AI tools in AI-only proxy mode by default.
- Improves Unix desktop launches so GUI apps detach and survive terminal shutdown.
- Improves `px init` handling for malformed existing `config.toml` files by stopping before discovery and leaving the file untouched.
