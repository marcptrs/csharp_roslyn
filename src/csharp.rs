use std::path::{Path, PathBuf};

use serde_json::json;
use url::Url;
use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct CsharpRoslynExtension;

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Find the roslyn-wrapper binary
        let (platform, _arch) = zed::current_platform();
        let wrapper_path = find_roslyn_wrapper(platform, worktree)?;

        // Run roslyn-wrapper (which will handle finding/downloading Roslyn)
        let mut env = worktree.shell_env();
        env.push(("ROSLYN_WRAPPER_CWD".into(), worktree.root_path()));
        Ok(zed::Command {
            command: wrapper_path,
            args: vec![],
            env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // Try to get solution path from settings first
        if let Some(solution_setting) = get_solution_path_from_settings(worktree) {
            if let Some(solution_uri) = resolve_solution_uri(&solution_setting, worktree) {
                return Ok(Some(json!({ "solution": solution_uri })));
            }
        }

        // Fallback: try to auto-detect solution
        if let Some(solution_path) = find_solution(worktree) {
            if let Some(solution_uri) = resolve_solution_uri(&solution_path, worktree) {
                return Ok(Some(json!({ "solution": solution_uri })));
            }
        }

        // No solution found - initialize without explicit solution
        // and let the wrapper handle project discovery
        Ok(Some(json!({})))
    }
}

/// Read solution path from user settings
fn get_solution_path_from_settings(worktree: &zed::Worktree) -> Option<String> {
    let settings = LspSettings::for_worktree("roslyn", worktree).ok()?;

    // Try to get solution_path from settings
    if let Some(init_options) = settings.initialization_options {
        if let Some(solution) = init_options.get("solution") {
            if let Some(solution_str) = solution.as_str() {
                return Some(solution_str.to_string());
            }
        }
    }

    None
}

/// Attempt to detect a solution file in a minimal, API-compatible way.
/// Currently returns None because Worktree doesn't support directory iteration.
fn find_solution(_worktree: &zed::Worktree) -> Option<String> {
    None
}

/// Convert file path to file:// URI
fn path_to_uri(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    match Url::from_file_path(path) {
        Ok(url) => url.into(),
        Err(_) => format!("file://{}", path.to_string_lossy().replace('\\', "/")),
    }
}

/// Find the roslyn-wrapper binary
fn find_roslyn_wrapper(platform: zed::Os, worktree: &zed::Worktree) -> Result<String> {
    let binary_name = match platform {
        zed::Os::Windows => "roslyn-wrapper.exe",
        _ => "roslyn-wrapper",
    };

    if let Some(path) = worktree.which(binary_name) {
        return Ok(path);
    }

    // Zed bundles binaries alongside the extension contents under `roslyn-wrapper/`
    Ok(format!("roslyn-wrapper/{}", binary_name))
}

fn resolve_solution_uri(value: &str, worktree: &zed::Worktree) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }

    if value.starts_with("file://") {
        return Some(value.to_string());
    }

    let mut candidate = PathBuf::from(value);
    if candidate.is_relative() {
        candidate = PathBuf::from(worktree.root_path()).join(candidate);
    }

    Some(path_to_uri(&candidate))
}
