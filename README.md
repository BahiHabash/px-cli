# px (Proxy Launcher)

`px` is a proxy-injector and application launcher. It intercepts your registered developer tools and seamlessly injects proxy configuration into their environment before launching them — via a **native shell wrapper** — making it easy to route AI/LLM traffic through a corporate or personal proxy without touching individual applications.

## Features

- **Auto-Discovery**: Scans your system for supported developer tools (Cursor, Codex, VS Code, etc.), resolves their executable paths (including Windows Store aliases), and automatically registers them.
- **Interactive Process Scanner**: When manually registering an app, `px` can scan your running processes and help you pick the exact executable path.
- **Shell-Wrapper Launcher**: Every app is launched via the platform's native shell (`/bin/sh` on Unix, `cmd` on Windows). The shell sets all env vars and then `exec`s the target app — no extra PID, no leaked processes.
- **AI-Only Proxy Mode**: Route *only* AI/LLM API traffic through the proxy. Non-AI traffic (npm, git, pip, crates.io, GitHub, etc.) bypasses the proxy entirely via `NO_PROXY`. Auto-enabled for Cursor and Codex.
- **Secure Credential Storage**: Stores proxy credentials locally in a `.env` file instead of committing them to version control.
- **Dynamic Config Overrides**: Reads `PX_HOST` and `PX_PORT` from your environment to dynamically override the proxy host/port.
- **App Execution Classes**: Supports both `cli` (blocking execution, inherits terminal IO) and `desktop` (detached execution, frees the terminal immediately).
- **Portable Configurations**: Saves paths as portable environment variables (e.g., `%LOCALAPPDATA%\...`) making the `config.toml` easily shareable.

## Setup & Installation

1. **Build the tool**: Run `cargo build --release` and place the executable (`px` or `px.exe`) in your system's PATH.
2. **Initialize**: Run `px init`.
   - Creates the platform configuration directory (e.g., `~/.config/proxy-launcher/` or `%APPDATA%\proxy-launcher\`).
   - Scaffolds a default `config.toml` and a `.env` template.
   - Runs the auto-discovery engine to find and register any supported tools — Cursor and Codex are automatically set to AI-only proxy mode.
3. **Configure Credentials**: Edit the generated `.env` file (located next to `config.toml`) and set your proxy details:
   ```env
   PX_PROXY_USER=your_username
   PX_PROXY_PASS=your_password
   PX_HOST=127.0.0.1
   PX_PORT=8080
   ```

## Usage

### Launching Apps
To launch a registered application with proxy variables injected:
```bash
px run <shortcut>
```
Example:
```bash
px run cursor-desktop   # launches with [ai-only] mode — only LLM API goes through proxy
px run codex-cli        # same — npm/git/pip bypass the proxy
```

You can also bypass the `.env` file for a specific run by providing a runtime override:
```bash
px run codex-cli --proxy-override "http://user:pass@10.0.0.1:8080"
```

### Managing Shortcuts

**Register a new app:**
```bash
px register --name my-app --path "C:\path\to\app.exe" --kind desktop
# With AI-only proxy mode:
px register --name cursor-desktop --path "/Applications/Cursor.app/Contents/MacOS/Cursor" --kind desktop --ai-only
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
| Unix     | `/bin/sh` | `sh -c 'HTTP_PROXY="..." exec /path/to/app'`    |
| Windows  | `cmd`     | `cmd /c "set HTTP_PROXY=...&& path\to\app.exe"` |

On Unix, `exec` *replaces* the shell process with the app — zero extra PIDs. On Windows, `cmd /c` exits as soon as the child process returns.

### Full Flow

1. `px` reads the `.env` file to retrieve `PX_PROXY_USER`, `PX_PROXY_PASS`, `PX_HOST`, and `PX_PORT`.
2. It constructs an HTTP proxy URL (e.g., `http://user:pass@127.0.0.1:8080`).
3. If `ai_only_proxy = true`, it also builds the `NO_PROXY` exclusion list.
4. It resolves the shortcut name against `config.toml` to find the exact path and arguments.
5. It builds a shell command string with all env vars exported inline.
6. It spawns the shell. Depending on the `kind` (`cli` vs `desktop`), it either blocks or detaches.
