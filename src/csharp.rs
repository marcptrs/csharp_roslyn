use std::path::{Path, PathBuf};

use serde_json::json;
use url::Url;
use zed_extension_api::{
    self as zed,
    settings::LspSettings,
    LanguageServerId, Result,
    DebugAdapterBinary, DebugTaskDefinition, StartDebuggingRequestArgumentsRequest,
    DebugConfig, DebugScenario, DebugRequest, TaskTemplate,
    StartDebuggingRequestArguments,
};

use crate::debugger;

pub struct CsharpRoslynExtension;

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let (platform, arch) = zed::current_platform();
        
        // Download wrapper (with progress reporting)
        let wrapper_path = crate::wrapper_download::ensure_wrapper(language_server_id, platform, arch, worktree)?;
        
        // Download Roslyn LSP (with progress reporting)
        let roslyn_path = crate::roslyn_download::ensure_roslyn(language_server_id, platform, arch, worktree)?;

        // Run wrapper with Roslyn binary path as argument
        let env = worktree.shell_env();
        Ok(zed::Command {
            command: wrapper_path,
            args: vec![roslyn_path],
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

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        _user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        if adapter_name != "netcoredbg" {
            return Err(format!("Unknown debug adapter: {}", adapter_name));
        }

        let command = debugger::ensure_debugger(worktree)?;
        
        // Parse the config JSON to get the request type
        let config_json: serde_json::Value = serde_json::from_str(&config.config)
            .map_err(|e| format!("Failed to parse config: {}", e))?;
        
        let request_type = config_json
            .get("request")
            .and_then(|v| v.as_str())
            .unwrap_or("launch");
        
        let request = match request_type {
            "attach" => StartDebuggingRequestArgumentsRequest::Attach,
            _ => StartDebuggingRequestArgumentsRequest::Launch,
        };
        
        Ok(DebugAdapterBinary {
            command: Some(command.command),
            arguments: command.args,
            envs: command.env,
            cwd: None,
            connection: None,
            request_args: StartDebuggingRequestArguments {
                configuration: config.config,
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
            _ => Err(format!("Unknown request type: {}", request_type)),
        }
    }

    fn dap_config_to_scenario(
        &mut self,
        config: DebugConfig,
    ) -> Result<DebugScenario, String> {
        // Extract launch request details
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

        // Only create debug scenarios for "run" related tasks
        // Check if this is a dotnet run/watch command
        let is_run_task = build_task.command.contains("dotnet") && 
            (build_task.command.contains("run") || 
             build_task.command.contains("watch") ||
             build_task.args.iter().any(|arg| arg == "run" || arg == "watch"));
        
        if !is_run_task {
            return None;
        }

        // For .NET debugging, we need to know the exact DLL path.
        // Try to infer it from the build task or use a generic path.
        
        // Try to infer project name from build task
        let program = if let Some(project_arg) = build_task.args.iter()
            .find(|arg| arg.ends_with(".csproj")) {
            // Extract project name from .csproj path
            let project_name = project_arg
                .trim_end_matches(".csproj")
                .split(['/', '\\'])
                .last()
                .unwrap_or("app");
            
            format!("$ZED_WORKTREE_ROOT/{}/bin/Debug/net9.0/{}.dll", 
                project_name, project_name)
        } else {
            // Fallback to a generic path
            "$ZED_WORKTREE_ROOT/bin/Debug/net9.0/app.dll".to_string()
        };
        
        let mut config = json!({
            "request": "launch",
            "program": program,
            "args": [],
            "cwd": "$ZED_WORKTREE_ROOT",
            "stopAtEntry": false,
            "console": "internalConsole"
        });

        // Ensure request field exists (required by DAP)
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