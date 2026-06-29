# px v2

`px` is a small proxy launcher for AI-heavy developer workflows. It launches registered tools with proxy environment variables injected, and AI-only mode keeps normal development traffic such as package managers, Git forges, and local services on a direct connection.

## How to install

Download the binary for your platform from this release:

- Linux: `px-linux`
- macOS Apple Silicon: `px-mac-silicon`
- macOS Intel: `px-mac-intel`
- Windows: `px-windows.exe`

Rename the downloaded Unix/macOS binary to `px`, mark it executable if needed, and place it somewhere in your `PATH`.

## How to run

```bash
px --help
px init
px credentials set
px check
px run <shortcut>
```

Typical AI shortcuts discovered by `px init` include `cursor-d`, `cursor`, `codex`, `kiro`, `claude`, and `antigravity`.

## Notes

- This release is SOCKS5-first. Credential-based runs generate proxy URLs like `socks5://user:pass@host:port`.
- `px` is mainly designed to handle AI work in developer tools by routing LLM/API traffic through a proxy.
- It does not provide full IDE control, does not cover every IDE action, and does not currently support MCP server routing or IDE extension marketplace workflows as first-class features.
- Some external network actions may not work 100% correctly yet, especially remote workflows such as `git push`, Git remote operations, and other tool-specific background network calls.

## Changes

- Adds `px credentials set` and `px credentials show` for local `.env` management.
- Adds shorter built-in shortcuts such as `cursor-d`, `codex`, `kiro`, `claude`, and `antigravity`.
- Expands auto-discovery for AI-focused developer tools.
- Keeps AI tools in AI-only proxy mode by default.
- Improves Unix desktop launches so GUI apps detach and survive terminal shutdown.
- Improves `px init` handling for malformed existing `config.toml` files by stopping before discovery and leaving the file untouched.
