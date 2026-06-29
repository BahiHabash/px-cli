//! no_proxy.rs — `NO_PROXY` / `no_proxy` exclusion list builder.
//!
//! When an app is launched in **AI-only proxy** mode, `px` sets `NO_PROXY` to a
//! broad exclusion list.  Any host NOT in that list routes through the proxy —
//! which, since we never include AI API endpoints, means all LLM/AI traffic is
//! automatically proxied.
//!
//! ## Built-in Baseline
//!
//! The baseline is read-only and always included.  It covers:
//!
//! | Category               | Examples                                    |
//! |------------------------|---------------------------------------------|
//! | Loopback / link-local  | `localhost`, `127.0.0.1`, `::1`             |
//! | Private IP ranges      | `10.*`, `172.16.*`, `192.168.*`             |
//! | Package registries     | `registry.npmjs.org`, `pypi.org`, `crates.io` |
//! | Git forges             | `github.com`, `gitlab.com`, `bitbucket.org` |
//! | Container registries   | `docker.io`, `ghcr.io`, `quay.io`           |
//! | Common CDN / update    | `cdn.jsdelivr.net`, `update.code.visualstudio.com` |
//!
//! ## AI hosts NOT in the baseline (intentionally routed through proxy)
//!
//! - `api.openai.com`
//! - `api.anthropic.com`
//! - `api.cohere.com`
//! - `generativelanguage.googleapis.com`  (Gemini)
//! - `api.mistral.ai`
//! - `api.groq.com`
//! - `api.perplexity.ai`
//! - `api.cursor.sh` / `api2.cursor.sh`
//!
//! ## `NO_PROXY` format compatibility
//!
//! We emit plain comma-separated hostnames and IP prefixes — the most portable
//! format understood by Node.js (Cursor, Codex), Python, curl, wget, and Go.
//! Wildcard-based exclusions (`*.example.com`) are intentionally avoided because
//! they are not universally supported.

// ---------------------------------------------------------------------------
// Built-in baseline — never AI API hosts, never changed by user config
// ---------------------------------------------------------------------------

/// Hosts that always bypass the proxy regardless of user configuration.
///
/// This list is deliberately conservative: if a host is absent the worst
/// outcome is that traffic is proxied unnecessarily (easy to add to
/// `no_proxy_extra`).  If a critical internal host is accidentally *included*
/// in the AI proxy exclusion list, legitimate AI traffic would leak direct.
static BASELINE: &[&str] = &[
    // ── Loopback / link-local ─────────────────────────────────────────────
    "localhost",
    "127.0.0.1",
    "127.0.0.0/8",
    "::1",
    "0.0.0.0",
    // ── Private RFC-1918 ranges ───────────────────────────────────────────
    // Listed as CIDR prefixes — Node.js and most tools accept these.
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    // ── npm / Node.js ─────────────────────────────────────────────────────
    "registry.npmjs.org",
    "npmjs.org",
    "npmjs.com",
    // ── Python ────────────────────────────────────────────────────────────
    "pypi.org",
    "files.pythonhosted.org",
    // ── Rust / Cargo ──────────────────────────────────────────────────────
    "crates.io",
    "static.crates.io",
    "doc.rust-lang.org",
    // ── Go modules ────────────────────────────────────────────────────────
    "proxy.golang.org",
    "sum.golang.org",
    "pkg.go.dev",
    // ── Git forges ────────────────────────────────────────────────────────
    "github.com",
    "api.github.com",
    "raw.githubusercontent.com",
    "objects.githubusercontent.com",
    "gitlab.com",
    "bitbucket.org",
    "sourceforge.net",
    // ── Container / package registries ────────────────────────────────────
    "docker.io",
    "registry-1.docker.io",
    "auth.docker.io",
    "ghcr.io",
    "quay.io",
    "gcr.io",
    "mcr.microsoft.com",
    // ── Microsoft & VS Code (telemetry, extensions, auth, updates) ────────
    "microsoft.com",
    ".microsoft.com",
    "visualstudio.com",
    ".visualstudio.com",
    "vscode.dev",
    ".vscode.dev",
    "vscode-cdn.net",
    ".vscode-cdn.net",
    "vsassets.io",
    ".vsassets.io",
    // ── CDN / update channels ─────────────────────────────────────────────
    "cdn.jsdelivr.net",
    "unpkg.com",
    "update.code.visualstudio.com",
    "marketplace.visualstudio.com",
    "vscode-cdn.net",
    "main.vscode-cdn.net",
    "vscode-unpkg.net",
    "gallerycdn.vsassets.io",
    "az764295.vo.msecnd.net", // VS Code update CDN
    "download.cursor.sh",     // Cursor self-update
    // ── OS / system services ──────────────────────────────────────────────
    "ocsp.apple.com",
    "swscan.apple.com",
    "swcdn.apple.com",
    "windowsupdate.microsoft.com",
    "download.microsoft.com",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Builds the `NO_PROXY` value by merging the built-in baseline with any
/// extra hosts supplied via `config.toml → [proxy] no_proxy_extra`.
///
/// The result is a comma-separated string suitable for the `NO_PROXY` /
/// `no_proxy` environment variable on all major platforms and runtimes.
///
/// # Example
///
/// ```
/// let no_proxy = no_proxy::build_no_proxy(&["git.acme.internal".to_string()]);
/// // → "localhost,127.0.0.1,...,git.acme.internal"
/// ```
pub fn build_no_proxy(extra: &[String]) -> String {
    let mut entries: Vec<&str> = BASELINE.to_vec();

    // Append user-supplied extras, deduplicating against the baseline.
    for host in extra {
        let host = host.trim();
        if !host.is_empty() && !BASELINE.contains(&host) {
            entries.push(host);
        }
    }

    entries.join(",")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_always_excluded() {
        let result = build_no_proxy(&[]);
        assert!(result.contains("localhost"));
        assert!(result.contains("127.0.0.1"));
    }

    #[test]
    fn ai_api_hosts_not_excluded() {
        let result = build_no_proxy(&[]);
        // These must NOT appear — they should route through the proxy.
        assert!(!result.contains("api.openai.com"));
        assert!(!result.contains("api.anthropic.com"));
        assert!(!result.contains("api.cursor.sh"));
        assert!(!result.contains("generativelanguage.googleapis.com"));
    }

    #[test]
    fn extra_hosts_appended() {
        let extra = vec![
            "git.acme.internal".to_string(),
            "registry.acme.internal".to_string(),
        ];
        let result = build_no_proxy(&extra);
        assert!(result.contains("git.acme.internal"));
        assert!(result.contains("registry.acme.internal"));
    }

    #[test]
    fn duplicates_not_added() {
        let extra = vec!["localhost".to_string()];
        let result = build_no_proxy(&extra);
        // "localhost" should appear exactly once.
        assert_eq!(result.matches("localhost").count(), 1);
    }

    #[test]
    fn output_is_comma_separated() {
        let result = build_no_proxy(&[]);
        assert!(!result.contains('\n'));
        assert!(!result.contains(' '));
        assert!(result.contains(','));
    }
}
