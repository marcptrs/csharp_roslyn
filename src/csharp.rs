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
        
        // Download OmniSharp-Roslyn (with progress reporting)
        eprintln!("[csharp_roslyn] Ensuring OmniSharp is available");
        let omnisharp_path = crate::omnisharp_download::ensure_omnisharp(
            language_server_id, 
            platform, 
            arch, 
            worktree
        )?;
        eprintln!("[csharp_roslyn] OmniSharp path: {}", omnisharp_path);

        // Run OmniSharp in LSP mode
        // OmniSharp will use the solution path from initialization_options
        // or auto-detect based on the working directory (worktree root)
        let root_path = worktree.root_path();
        eprintln!("[csharp_roslyn] Worktree root: {}", root_path);
        
        let env = worktree.shell_env();
        
        eprintln!("[csharp_roslyn] Starting OmniSharp with -lsp flag");
        
        Ok(zed::Command {
            command: omnisharp_path,
            args: vec![
                "-lsp".to_string(),
            ],
            env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // Build base initialization options with Roslyn extensions
        let mut init_options = json!({
            "RoslynExtensionsOptions": {
                "enableDecompilationSupport": true,
                "enableImportCompletion": true,
                "enableAnalyzersSupport": true
            }
        });

        // Try to get solution path from settings first
        if let Some(solution_setting) = get_solution_path_from_settings(worktree) {
            eprintln!("[csharp_roslyn] Found solution in settings: {}", solution_setting);
            if let Some(solution_uri) = resolve_solution_uri(&solution_setting, worktree) {
                eprintln!("[csharp_roslyn] Resolved solution URI: {}", solution_uri);
                init_options["solution"] = json!(solution_uri);
                return Ok(Some(init_options));
            }
        }

        // Fallback: try to auto-detect solution
        if let Some(solution_path) = find_solution(worktree) {
            eprintln!("[csharp_roslyn] Auto-detected solution: {}", solution_path);
            if let Some(solution_uri) = resolve_solution_uri(&solution_path, worktree) {
                eprintln!("[csharp_roslyn] Resolved solution URI: {}", solution_uri);
                init_options["solution"] = json!(solution_uri);
                return Ok(Some(init_options));
            }
        }

        // Return initialization options even without solution
        eprintln!("[csharp_roslyn] Returning init options with decompilation support enabled");
        Ok(Some(init_options))
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
        
        // Try to infer project path and name from build task
        let program = if let Some(project_arg) = build_task.args.iter()
            .find(|arg| arg.ends_with(".csproj")) {
            // Extract project directory and name from .csproj path
            // e.g., "src/ConsoleApp/ConsoleApp.csproj" -> directory="src/ConsoleApp", name="ConsoleApp"
            
            // Get the parent directory path (everything before the .csproj filename)
            let project_dir = if let Some(last_slash) = project_arg.rfind(['/', '\\']) {
                &project_arg[..last_slash]
            } else {
                "."
            };
            
            // Get the project name from the .csproj filename
            let project_name = project_arg
                .split(['/', '\\'])
                .last()
                .and_then(|s| s.strip_suffix(".csproj"))
                .unwrap_or("app");
            
            // Use forward slashes in the path template - Zed will normalize when expanding $ZED_WORKTREE_ROOT
            format!("$ZED_WORKTREE_ROOT/{}/bin/Debug/net9.0/{}.dll", 
                project_dir.replace('\\', "/"), project_name)
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
    let settings = LspSettings::for_worktree("omnisharp-roslyn", worktree).ok()?;

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

/// Attempt to detect a solution file in the worktree root.
/// Since we can't use std::fs in WASM, we return None to let OmniSharp auto-detect.
fn find_solution(worktree: &zed::Worktree) -> Option<String> {
    let root_path = worktree.root_path();
    eprintln!("[csharp_roslyn] Cannot enumerate files in WASM sandbox");
    eprintln!("[csharp_roslyn] Returning None - OmniSharp will auto-detect from working directory: {}", root_path);
    
    // Return None so OmniSharp auto-detects the solution from its working directory
    // The working directory is correctly set to the worktree root by language_server_command
    None
}

/// Convert file path to file:// URI
fn path_to_uri(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    match Url::from_file_path(path) {
        Ok(url) => url.to_string(),
        Err(_) => {
            // Fallback: manually construct file URI with proper formatting
            let path_str = path.to_string_lossy().replace('\\', "/");
            // Ensure we have three slashes for absolute paths on Windows (file:///C:/...)
            if path_str.starts_with('/') || path_str.chars().nth(1) == Some(':') {
                format!("file:///{}", path_str.trim_start_matches('/'))
            } else {
                format!("file://{}", path_str)
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