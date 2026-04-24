use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use serde_json::json;
use url::Url;
use zed_extension_api::{
    self as zed, settings::LspSettings, DebugAdapterBinary, DebugConfig, DebugRequest,
    DebugScenario, DebugTaskDefinition, LanguageServerId, Result, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TaskTemplate,
};

use crate::debugger;
use crate::language_servers::{Omnisharp, Roslyn};
use crate::logging::debug_log;
use crate::project_info::{
    ensure_unity_project_files, get_unity_omnisharp_config, is_unity_project, DotNetProject,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BackendKind {
    Roslyn,
    Omnisharp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BackendReason {
    ExplicitOmnisharpRequest,
    ImplicitRoslynSelection,
    CustomRoslynBinaryOverride,
    UnityProject,
    DotNet10RuntimeDetected { version: String, root: PathBuf },
    DotNet10RuntimeMissing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BackendSelection {
    kind: BackendKind,
    reason: BackendReason,
}

pub struct CsharpRoslynExtension {
    omnisharp: Option<Omnisharp>,
    roslyn: Option<Roslyn>,
}

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self {
            omnisharp: None,
            roslyn: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match self.backend_for_server(language_server_id, worktree).kind {
            BackendKind::Roslyn => {
                debug_log!(worktree, "[csharp_roslyn] Starting roslyn-language-server");
                let roslyn = self.roslyn.get_or_insert_with(Roslyn::new);
                let mut command = roslyn.language_server_cmd(language_server_id, worktree)?;
                command.env = merged_lsp_env(worktree, Roslyn::LANGUAGE_SERVER_ID);
                Ok(command)
            }
            BackendKind::Omnisharp => {
                debug_log!(worktree, "[csharp_roslyn] Starting OmniSharp");
                let omnisharp = self.omnisharp.get_or_insert_with(Omnisharp::new);
                let binary = omnisharp.language_server_binary(language_server_id, worktree)?;

                Ok(zed::Command {
                    command: binary.path,
                    args: binary.args.unwrap_or_else(|| vec!["-lsp".to_string()]),
                    env: merged_lsp_env(worktree, Omnisharp::LANGUAGE_SERVER_ID),
                })
            }
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        match self.backend_for_server(language_server_id, worktree).kind {
            BackendKind::Roslyn => Ok(None),
            BackendKind::Omnisharp => omnisharp_initialization_options(worktree),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        match self.backend_for_server(language_server_id, worktree).kind {
            BackendKind::Roslyn => Roslyn::configuration_options(worktree),
            BackendKind::Omnisharp => {
                LspSettings::for_worktree(Omnisharp::LANGUAGE_SERVER_ID, worktree)
                    .map(|settings| settings.settings)
            }
        }
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        _user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        if adapter_name != "netcoredbg" {
            return Err(format!("Unknown debug adapter: {adapter_name}"));
        }

        let command = debugger::ensure_debugger(worktree)?;

        let mut config_json: serde_json::Value = serde_json::from_str(&config.config)
            .map_err(|e| format!("Failed to parse config: {e}"))?;

        let request_type = config_json
            .get("request")
            .and_then(|v| v.as_str())
            .unwrap_or("launch");

        let request = match request_type {
            "attach" => StartDebuggingRequestArgumentsRequest::Attach,
            _ => StartDebuggingRequestArgumentsRequest::Launch,
        };

        if let Some(program_value) = config_json.get_mut("program") {
            if let Some(program_str) = program_value.as_str() {
                if program_str.contains("$TARGET_FRAMEWORK") && program_str.contains("/bin/Debug/")
                {
                    let worktree_root = worktree.root_path();
                    let rel = if program_str.starts_with(&worktree_root) {
                        program_str
                            .trim_start_matches(&worktree_root)
                            .trim_start_matches('/')
                    } else {
                        program_str
                    };

                    let parts: Vec<&str> = rel.split('/').collect();
                    if let Some(bin_idx) = parts.iter().position(|p| *p == "bin") {
                        let project_dir = if bin_idx == 0 {
                            ".".to_string()
                        } else {
                            parts[..bin_idx].join("/")
                        };

                        if let Some(file_name) = parts.last() {
                            if let Some(name) = file_name
                                .strip_suffix(".dll")
                                .or_else(|| file_name.strip_suffix(".exe"))
                            {
                                let csproj_path = if project_dir == "." {
                                    format!("{name}.csproj")
                                } else {
                                    format!("{project_dir}/{name}.csproj")
                                };

                                if let Ok(text) = worktree.read_text_file(&csproj_path) {
                                    let project = DotNetProject::from_csproj_text(
                                        &text,
                                        Path::new(&csproj_path),
                                    );
                                    let new_program = program_str
                                        .replace("$TARGET_FRAMEWORK", &project.target_framework);
                                    *program_value = serde_json::Value::String(new_program);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(DebugAdapterBinary {
            command: Some(command.command),
            arguments: command.args,
            envs: command.env,
            cwd: None,
            connection: None,
            request_args: StartDebuggingRequestArguments {
                configuration: serde_json::to_string(&config_json)
                    .map_err(|e| format!("Failed to serialize modified config: {e}"))?,
                request,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        config: serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        let request_type = config
            .get("request")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'request' field in debug configuration".to_string())?;

        match request_type {
            "launch" => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            "attach" => Ok(StartDebuggingRequestArgumentsRequest::Attach),
            _ => Err(format!("Unknown request type: {request_type}")),
        }
    }

    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario, String> {
        let (program, args, cwd, envs) = match &config.request {
            DebugRequest::Launch(launch) => (
                launch.program.clone(),
                launch.args.clone(),
                launch.cwd.clone().unwrap_or_else(|| ".".to_string()),
                launch.envs.clone(),
            ),
            DebugRequest::Attach(_) => {
                return Err("Attach requests not yet supported".to_string());
            }
        };

        let launch_config = json!({
            "request": "launch",
            "program": program,
            "args": args,
            "cwd": cwd,
            "env": envs,
            "stopAtEntry": config.stop_on_entry.unwrap_or(false),
            "console": "internalConsole",
        });

        Ok(DebugScenario {
            label: config.label,
            adapter: config.adapter,
            build: None,
            config: launch_config.to_string(),
            tcp_connection: None,
        })
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<DebugScenario> {
        if debug_adapter_name != "netcoredbg" || locator_name != "dotnet" {
            return None;
        }

        let is_run_task = build_task.command.contains("dotnet")
            && (build_task.command.contains("run")
                || build_task.command.contains("watch")
                || build_task
                    .args
                    .iter()
                    .any(|arg| arg == "run" || arg == "watch"));

        if !is_run_task {
            return None;
        }

        let program = if let Some(project_arg) =
            build_task.args.iter().find(|arg| arg.ends_with(".csproj"))
        {
            let project_dir = if let Some(last_slash) = project_arg.rfind(['/', '\\']) {
                &project_arg[..last_slash]
            } else {
                "."
            };

            let project_name = project_arg
                .split(['/', '\\'])
                .next_back()
                .and_then(|s| s.strip_suffix(".csproj"))
                .unwrap_or("app");
            format!(
                "$ZED_WORKTREE_ROOT/{}/bin/Debug/$TARGET_FRAMEWORK/{}.dll",
                project_dir.replace('\\', "/"),
                project_name
            )
        } else {
            "$ZED_WORKTREE_ROOT/bin/Debug/$TARGET_FRAMEWORK/app.dll".to_string()
        };

        let mut config = json!({
            "request": "launch",
            "program": program,
            "args": [],
            "cwd": "$ZED_WORKTREE_ROOT",
            "stopAtEntry": false,
            "console": "internalConsole"
        });

        if let Some(obj) = config.as_object_mut() {
            obj.entry("request").or_insert("launch".into());
        }

        Some(DebugScenario {
            adapter: debug_adapter_name,
            label: resolved_label,
            config: config.to_string(),
            tcp_connection: None,
            build: None,
        })
    }
}

impl CsharpRoslynExtension {
    fn backend_for_server(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> BackendSelection {
        match language_server_id.as_ref() {
            Omnisharp::LANGUAGE_SERVER_ID => BackendSelection {
                kind: BackendKind::Omnisharp,
                reason: BackendReason::ExplicitOmnisharpRequest,
            },
            Roslyn::LANGUAGE_SERVER_ID => {
                let selection = select_backend_for_worktree(worktree);
                log_backend_selection(worktree, &selection);
                selection
            }
            _ => BackendSelection {
                kind: BackendKind::Roslyn,
                reason: BackendReason::ImplicitRoslynSelection,
            },
        }
    }
}

fn merged_lsp_env(worktree: &zed::Worktree, language_server_id: &str) -> Vec<(String, String)> {
    let mut env = worktree.shell_env();

    fn set_env_var(env: &mut Vec<(String, String)>, key: &str, value: String) {
        for (existing_key, existing_value) in env.iter_mut() {
            if existing_key == key {
                *existing_value = value;
                return;
            }
        }
        env.push((key.to_string(), value));
    }

    if let Ok(host_dotnet_root) = std::env::var("DOTNET_ROOT") {
        if !host_dotnet_root.is_empty() {
            set_env_var(&mut env, "DOTNET_ROOT", host_dotnet_root);
        }
    }
    if let Ok(host_path) = std::env::var("PATH") {
        if !host_path.is_empty() {
            set_env_var(&mut env, "PATH", host_path);
        }
    }

    if let Ok(settings) = LspSettings::for_worktree(language_server_id, worktree) {
        if let Some(binary) = settings.binary {
            if let Some(extra_env) = binary.env {
                for (key, value) in extra_env {
                    set_env_var(&mut env, &key, value);
                }
            }
        }
    }

    env
}

fn select_backend_for_worktree(worktree: &zed::Worktree) -> BackendSelection {
    if roslyn_binary_is_overridden(worktree) {
        return BackendSelection {
            kind: BackendKind::Roslyn,
            reason: BackendReason::CustomRoslynBinaryOverride,
        };
    }

    if is_unity_project(worktree) {
        return BackendSelection {
            kind: BackendKind::Omnisharp,
            reason: BackendReason::UnityProject,
        };
    }

    if let Some((version, root)) = find_dotnet_runtime_at_least(worktree, 10) {
        return BackendSelection {
            kind: BackendKind::Roslyn,
            reason: BackendReason::DotNet10RuntimeDetected { version, root },
        };
    }

    BackendSelection {
        kind: BackendKind::Omnisharp,
        reason: BackendReason::DotNet10RuntimeMissing,
    }
}

fn log_backend_selection(worktree: &zed::Worktree, selection: &BackendSelection) {
    match &selection.reason {
        BackendReason::ExplicitOmnisharpRequest => {
            debug_log!(
                worktree,
                "[csharp_roslyn] OmniSharp explicitly requested by Zed/server selection"
            )
        }
        BackendReason::ImplicitRoslynSelection => {
            debug_log!(
                worktree,
                "[csharp_roslyn] Using Roslyn for implicit server selection"
            )
        }
        BackendReason::CustomRoslynBinaryOverride => {
            debug_log!(
                worktree,
                "[csharp_roslyn] Using Roslyn because a custom roslyn binary path is configured"
            )
        }
        BackendReason::UnityProject => {
            debug_log!(
                worktree,
                "[csharp_roslyn] Falling back to OmniSharp because this appears to be a Unity project"
            )
        }
        BackendReason::DotNet10RuntimeDetected { version, root } => {
            debug_log!(
                worktree,
                "[csharp_roslyn] Using Roslyn because .NET runtime {version} was detected under {}",
                root.display()
            )
        }
        BackendReason::DotNet10RuntimeMissing => {
            debug_log!(
                worktree,
                "[csharp_roslyn] Falling back to OmniSharp because no .NET 10+ runtime was detected in any known DOTNET_ROOT/shared runtime location"
            )
        }
    }
}

fn roslyn_binary_is_overridden(worktree: &zed::Worktree) -> bool {
    LspSettings::for_worktree(Roslyn::LANGUAGE_SERVER_ID, worktree)
        .ok()
        .and_then(|settings| settings.binary)
        .and_then(|binary| binary.path)
        .is_some()
}

fn find_dotnet_runtime_at_least(
    worktree: &zed::Worktree,
    required_major: u64,
) -> Option<(String, PathBuf)> {
    let mut best_match: Option<(String, PathBuf)> = None;

    for dotnet_root in dotnet_root_candidates(worktree) {
        let runtime_root = dotnet_root.join("shared").join("Microsoft.NETCore.App");
        let Ok(entries) = std::fs::read_dir(&runtime_root) else {
            continue;
        };

        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(|value| value.to_string()) else {
                continue;
            };

            let major = name
                .split('.')
                .next()
                .and_then(|segment| segment.parse::<u64>().ok());
            if major.is_some_and(|major| major >= required_major) {
                match &best_match {
                    Some((best_version, _))
                        if compare_version_like_strings(&name, best_version).is_le() => {}
                    _ => best_match = Some((name, dotnet_root.clone())),
                }
            }
        }
    }

    best_match
}

fn compare_version_like_strings(left: &str, right: &str) -> Ordering {
    let parse = |value: &str| {
        value
            .split(['.', '-'])
            .map(|segment| segment.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };

    let left = parse(left);
    let right = parse(right);
    let max_len = left.len().max(right.len());

    for index in 0..max_len {
        let left_part = *left.get(index).unwrap_or(&0);
        let right_part = *right.get(index).unwrap_or(&0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => continue,
            ordering => return ordering,
        }
    }

    Ordering::Equal
}

fn dotnet_root_candidates(worktree: &zed::Worktree) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let (os, _) = zed::current_platform();

    for key in ["DOTNET_ROOT", "DOTNET_ROOT(x86)"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                candidates.push(PathBuf::from(value));
            }
        }

        if let Some(value) = shell_env_var(worktree, key) {
            if !value.is_empty() {
                candidates.push(PathBuf::from(value));
            }
        }
    }

    if let Ok(path) = std::env::var("PATH") {
        candidates.extend(dotnet_roots_from_path_env(&path, os));
    }
    if let Some(path) = shell_env_var(worktree, "PATH") {
        candidates.extend(dotnet_roots_from_path_env(&path, os));
    }

    if let Some(dotnet_path) = worktree.which("dotnet") {
        candidates.extend(dotnet_roots_from_binary_path(Path::new(&dotnet_path)));
    }

    match os {
        zed::Os::Mac => {
            candidates.push(PathBuf::from("/opt/homebrew/share/dotnet"));
            candidates.push(PathBuf::from("/usr/local/share/dotnet"));
        }
        zed::Os::Linux => {
            candidates.push(PathBuf::from("/usr/share/dotnet"));
            candidates.push(PathBuf::from("/usr/local/share/dotnet"));
        }
        zed::Os::Windows => {
            candidates.push(PathBuf::from(r"C:\Program Files\dotnet"));
            candidates.push(PathBuf::from(r"C:\Program Files (x86)\dotnet"));
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn shell_env_var(worktree: &zed::Worktree, key: &str) -> Option<String> {
    worktree
        .shell_env()
        .into_iter()
        .find_map(|(name, value)| (name == key).then_some(value))
}

fn dotnet_roots_from_path_env(path_env: &str, os: zed::Os) -> Vec<PathBuf> {
    let separator = if os == zed::Os::Windows { ';' } else { ':' };

    path_env
        .split(separator)
        .filter(|entry| !entry.is_empty())
        .flat_map(|entry| {
            let candidate = PathBuf::from(entry);
            let mut roots = Vec::new();

            if entry.to_ascii_lowercase().contains("dotnet") {
                roots.push(candidate.clone());
                if let Some(parent) = candidate.parent() {
                    roots.push(parent.to_path_buf());
                }
            }

            roots
        })
        .collect()
}

fn dotnet_roots_from_binary_path(dotnet_binary: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(parent) = dotnet_binary.parent() {
        roots.push(parent.to_path_buf());

        if parent
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
        {
            if let Some(parent_of_bin) = parent.parent() {
                roots.push(parent_of_bin.to_path_buf());
            }
        }
    }

    if let Ok(canonical) = std::fs::canonicalize(dotnet_binary) {
        if let Some(parent) = canonical.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    roots
}

fn omnisharp_initialization_options(worktree: &zed::Worktree) -> Result<Option<serde_json::Value>> {
    if is_unity_project(worktree) {
        debug_log!(
            worktree,
            "[csharp_roslyn] Unity project detected for OmniSharp"
        );
        return match ensure_unity_project_files(worktree) {
            Ok(solution_path) => {
                let mut unity_config = get_unity_omnisharp_config();
                if let Some(solution_uri) = resolve_solution_uri(&solution_path, worktree) {
                    unity_config["solution"] = json!(solution_uri);
                }
                Ok(Some(unity_config))
            }
            Err(instructions) => {
                debug_log!(worktree, "[csharp_roslyn] {instructions}");
                Ok(Some(get_unity_omnisharp_config()))
            }
        };
    }

    let mut init_options = json!({
        "RoslynExtensionsOptions": {
            "enableDecompilationSupport": true,
            "enableImportCompletion": true,
            "enableAnalyzersSupport": true
        }
    });

    if let Some(solution_path) = detect_solution_path(worktree) {
        if let Some(solution_uri) = resolve_solution_uri(&solution_path, worktree) {
            debug_log!(
                worktree,
                "[csharp_roslyn] Using OmniSharp solution/workspace path: {solution_path}"
            );
            init_options["solution"] = json!(solution_uri);
        }
    } else {
        debug_log!(
            worktree,
            "[csharp_roslyn] No explicit solution file detected for OmniSharp; allowing server-side project auto-discovery"
        );
    }

    Ok(Some(init_options))
}

fn detect_solution_path(worktree: &zed::Worktree) -> Option<String> {
    if let Some(solution_path) = get_solution_path_from_settings(worktree) {
        debug_log!(
            worktree,
            "[csharp_roslyn] Found OmniSharp solution path in settings: {solution_path}"
        );
        return Some(solution_path);
    }

    let root_path = worktree.root_path();
    let root_name = Path::new(&root_path)
        .file_name()
        .and_then(|name| name.to_str())?;

    for extension in ["sln", "slnx", "slnf"] {
        let candidate = format!("{root_name}.{extension}");
        if worktree.read_text_file(&candidate).is_ok() {
            debug_log!(
                worktree,
                "[csharp_roslyn] Found workspace-matching solution file: {candidate}"
            );
            return Some(candidate);
        }
    }

    None
}

fn get_solution_path_from_settings(worktree: &zed::Worktree) -> Option<String> {
    let settings = LspSettings::for_worktree(Omnisharp::LANGUAGE_SERVER_ID, worktree).ok()?;
    let init_options = settings.initialization_options?;
    let solution = init_options.get("solution")?;
    solution.as_str().map(ToString::to_string)
}

fn path_to_uri(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    match Url::from_file_path(path) {
        Ok(url) => url.to_string(),
        Err(_) => {
            let path_str = path.to_string_lossy().replace('\\', "/");
            if path_str.starts_with('/') || path_str.chars().nth(1) == Some(':') {
                format!("file:///{}", path_str.trim_start_matches('/'))
            } else {
                format!("file://{path_str}")
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_version_like_strings_prefers_higher_versions() {
        assert_eq!(
            compare_version_like_strings("10.0.1", "10.0.0"),
            Ordering::Greater
        );
        assert_eq!(
            compare_version_like_strings("10.0.0", "10.0.1"),
            Ordering::Less
        );
        assert_eq!(
            compare_version_like_strings("10.0.0", "10.0.0"),
            Ordering::Equal
        );
        assert_eq!(
            compare_version_like_strings("10.0.0-preview.2", "10.0.0-preview.1"),
            Ordering::Greater
        );
    }
}
