# px (Proxy Launcher)

`px` is a proxy-injector and application launcher. It intercepts your registered developer tools and seamlessly injects proxy configuration into their environment before launching them, making it easy to route traffic through corporate or personal proxies without hardcoding credentials into individual applications.

## Features

- **Auto-Discovery**: Scans your system for supported developer tools (like VS Code, Cursor, etc.), resolves their executable paths (including Windows Store aliases), and automatically registers them.
- **Interactive Process Scanner**: When manually registering an app, `px` can scan your running processes and help you pick the exact executable path if you don't know it.
- **Environment Variable Injection**: Automatically injects `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` variables into the child process.
- **Secure Credential Storage**: Stores proxy credentials locally in a `.env` file instead of committing them to version control.
- **Dynamic Config Overrides**: Reads `PX_HOST` and `PX_PORT` from your environment to dynamically override the proxy host/port.
- **App Execution Classes**: Supports both `cli` (blocking execution, inherits terminal IO) and `desktop` (detached execution, frees the terminal immediately).
- **Portable Configurations**: Saves paths as portable environment variables (e.g., `%LOCALAPPDATA%\...`) making the `config.toml` easily shareable.

## Setup & Installation

1. **Build the tool**: Run `cargo build --release` and place the executable (`px` or `px.exe`) in your system's PATH.
2. **Initialize**: Run `px init`.
   - This creates the platform configuration directory (e.g., `~/.config/proxy-launcher/` or `%APPDATA%\proxy-launcher\`).
   - It scaffolds a default `config.toml` and a `.env` template.
   - It runs the auto-discovery engine to find and register any supported tools already on your machine.
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
px run cursor-desktop
```

You can also bypass the `.env` file for a specific run by providing a runtime override:
```bash
px run codex-cli --proxy-override "http://user:pass@10.0.0.1:8080"
```

### Managing Shortcuts

**Register a new app:**
```bash
px register --name my-app --path "C:\path\to\app.exe" --kind desktop
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

## How It Works

1. `px` reads the `.env` file to retrieve `PX_PROXY_USER`, `PX_PROXY_PASS`, `PX_HOST`, and `PX_PORT`.
2. It constructs an HTTP proxy URL (e.g., `http://user:pass@127.0.0.1:8080`).
3. It resolves the shortcut name against `config.toml` to find the exact path to the executable and its arguments.
4. It spawns the application with the `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` variables securely injected. Depending on the `kind` of application (`cli` vs `desktop`), it will either block and wait for the process to exit, or detach it immediately.
