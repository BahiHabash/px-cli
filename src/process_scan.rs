//! process_scan.rs — Cross-platform running-process inspection.
//!
//! The scanner is built on `sysinfo` so it works across macOS, Linux, and
//! Windows.  It filters noisy Electron helper processes, detects known tools
//! through `app_registry`, and normalizes locked Windows Store paths when the
//! process manager exposes an unlaunchable binary.

use std::collections::HashSet;
use std::path::Path;

use sysinfo::{Pid, Process, System};

use crate::app_registry::{self, AppDefinition};
use crate::config::AppKind;

/// A running process that can potentially be registered as an app.
#[derive(Debug, Clone)]
pub struct ProcessMatch {
    pub pid: Pid,
    pub process_name: String,
    pub path: String,
    pub args: Vec<String>,
    pub detected_shortcut: Option<&'static str>,
    pub detected_name: Option<&'static str>,
    pub kind: AppKind,
    pub ai_only_proxy: bool,
}

impl ProcessMatch {
    pub fn from_process(process: &Process) -> Option<Self> {
        if is_ignored_process(process) {
            return None;
        }

        let exe = process.exe()?;
        let process_name = process.name().to_string();
        let raw_path = normalize_process_path(exe, process.cwd());
        let detected = app_registry::identify_process(&process_name, &raw_path);
        let path = launchable_path(&raw_path, detected);
        let args = process.cmd().iter().skip(1).cloned().collect();

        Some(Self {
            pid: process.pid(),
            process_name,
            path,
            args,
            detected_shortcut: detected.map(|app| app.shortcut),
            detected_name: detected.map(|app| app.display_name),
            kind: detected.map(|app| app.kind).unwrap_or(AppKind::Desktop),
            ai_only_proxy: detected.map(|app| app.ai_only_proxy).unwrap_or(false),
        })
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.process_name.to_lowercase().contains(&query)
            || self.path.to_lowercase().contains(&query)
            || self
                .detected_shortcut
                .map(|shortcut| shortcut.to_lowercase().contains(&query))
                .unwrap_or(false)
            || self
                .detected_name
                .map(|name| name.to_lowercase().contains(&query))
                .unwrap_or(false)
    }
}

/// Returns true if this process is an Electron renderer / gpu / crashpad
/// subprocess rather than the main app process.
pub fn is_electron_subprocess(process: &Process) -> bool {
    process
        .cmd()
        .iter()
        .skip(1)
        .any(|a| a.starts_with("--type="))
}

/// Additional helper guard for macOS and Chromium/Electron apps whose helper
/// processes sometimes do not expose `--type=`.
pub fn is_helper_process(process: &Process) -> bool {
    let name = process.name().to_lowercase();
    let exe = process
        .exe()
        .map(|path| path.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    name.contains("helper")
        || name.contains("crashpad")
        || exe.contains("helper")
        || exe.contains("crashpad")
}

pub fn is_ignored_process(process: &Process) -> bool {
    process.exe().is_none() || is_electron_subprocess(process) || is_helper_process(process)
}

/// Snapshot process ids before an interactive registration prompt.
pub fn snapshot_pids() -> HashSet<Pid> {
    let sys = System::new_all();
    sys.processes().keys().copied().collect()
}

/// Returns all registerable running processes.
pub fn all_registerable() -> Vec<ProcessMatch> {
    let sys = System::new_all();
    collect_from_system(&sys, |_| true)
}

/// Returns registerable processes matching a user query.
pub fn search(query: &str) -> Vec<ProcessMatch> {
    all_registerable()
        .into_iter()
        .filter(|process| process.matches_query(query))
        .collect()
}

/// Returns registerable processes that appeared after `before`.
pub fn new_since(before: &HashSet<Pid>) -> Vec<ProcessMatch> {
    let sys = System::new_all();
    collect_from_system(&sys, |process| !before.contains(&process.pid()))
}

/// Finds currently running known desktop apps.
pub fn known_running(apps: &[&'static AppDefinition]) -> Vec<ProcessMatch> {
    let sys = System::new_all();
    let mut found = Vec::new();
    let mut seen_shortcuts = HashSet::new();

    for process in collect_from_system(&sys, |_| true) {
        let Some(shortcut) = process.detected_shortcut else {
            continue;
        };

        if !apps.iter().any(|app| app.shortcut == shortcut) {
            continue;
        }

        if seen_shortcuts.insert(shortcut) {
            found.push(process);
        }
    }

    found
}

fn collect_from_system<F>(sys: &System, mut keep: F) -> Vec<ProcessMatch>
where
    F: FnMut(&Process) -> bool,
{
    let mut processes: Vec<_> = sys
        .processes()
        .values()
        .filter(|process| keep(process))
        .filter_map(ProcessMatch::from_process)
        .collect();

    processes.sort_by(|a, b| {
        a.process_name
            .to_lowercase()
            .cmp(&b.process_name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });

    processes
}

fn normalize_process_path(exe_path: &Path, cwd: Option<&Path>) -> String {
    if exe_path.is_absolute() {
        return exe_path.to_string_lossy().into_owned();
    }

    cwd.unwrap_or_else(|| Path::new(""))
        .join(exe_path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(target_os = "windows")]
pub fn is_locked_store_path(path_lower: &str) -> bool {
    path_lower.contains("\\windowsapps\\")
        && !path_lower.contains("\\local\\microsoft\\windowsapps\\")
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn is_locked_store_path(_path_lower: &str) -> bool {
    false
}

fn launchable_path(raw_path: &str, detected: Option<&AppDefinition>) -> String {
    #[cfg(target_os = "windows")]
    {
        let path_lower = raw_path.to_lowercase();
        if is_locked_store_path(&path_lower) {
            if let Some(app) = detected {
                if let Some(alias) = windows_store_alias(app.shortcut) {
                    return alias.to_string();
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    let _ = detected;

    raw_path.to_string()
}

#[cfg(target_os = "windows")]
fn windows_store_alias(shortcut: &str) -> Option<&'static str> {
    match shortcut {
        "vscode-d" => Some(r"%LOCALAPPDATA%\Microsoft\WindowsApps\code.exe"),
        "codex-d" | "codex" => Some(r"%LOCALAPPDATA%\Microsoft\WindowsApps\codex.exe"),
        "kiro-d" | "kiro" => Some(r"%LOCALAPPDATA%\Microsoft\WindowsApps\kiro.exe"),
        "claude-d" | "claude" => Some(r"%LOCALAPPDATA%\Microsoft\WindowsApps\claude.exe"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_store_paths_are_not_locked_on_this_target() {
        #[cfg(not(target_os = "windows"))]
        assert!(!is_locked_store_path(
            r"c:\program files\windowsapps\codex.exe"
        ));
    }

    #[test]
    fn relative_process_paths_are_joined_to_cwd() {
        let result = normalize_process_path(
            Path::new("target/release/px"),
            Some(Path::new("/Users/example/project")),
        );
        assert_eq!(result, "/Users/example/project/target/release/px");
    }
}
