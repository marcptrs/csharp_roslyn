use std::path::{Path, PathBuf};

use serde_json::json;
use url::Url;
use zed_extension_api::{
    self as zed, settings::LspSettings, DebugAdapterBinary, DebugConfig, DebugRequest,
    DebugScenario, DebugTaskDefinition, LanguageServerId, Result, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TaskTemplate,
};

use crate::debugger;
use crate::project_info::DotNetProject;

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
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Ensuring OmniSharp is available"); }
        let omnisharp_path = crate::omnisharp_download::ensure_omnisharp(
            language_server_id,
            platform,
            arch,
            worktree,
        )?;
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] OmniSharp path: {}", omnisharp_path); }

        // Run OmniSharp in LSP mode
        // OmniSharp will use the solution path from initialization_options
        // or auto-detect based on the working directory (worktree root)
        let root_path = worktree.root_path();
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Worktree root: {}", root_path); }

        let mut env = worktree.shell_env();
        // Ensure DOTNET_ROOT and PATH come from the host environment so OmniSharp uses the same SDK/tools
        fn set_env_var(env: &mut Vec<(String, String)>, key: &str, value: String) {
            for (k, v) in env.iter_mut() {
                if k == key {
                    *v = value;
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

        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Starting OmniSharp with -lsp flag"); }

        Ok(zed::Command {
            command: omnisharp_path,
            args: vec!["-lsp".to_string()],
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
            if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Found solution in settings: {}", solution_setting); }
            if let Some(solution_uri) = resolve_solution_uri(&solution_setting, worktree) {
                if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Resolved solution URI: {}", solution_uri); }
                init_options["solution"] = json!(solution_uri);
                return Ok(Some(init_options));
            }
        }

        // Fallback: try to auto-detect solution
        if let Some(solution_path) = find_solution(worktree) {
            if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Auto-detected solution: {}", solution_path); }
            if let Some(solution_uri) = resolve_solution_uri(&solution_path, worktree) {
                if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Resolved solution URI: {}", solution_uri); }
                init_options["solution"] = json!(solution_uri);
                return Ok(Some(init_options));
            }
        }

        // Return initialization options even without solution
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Returning init options with decompilation support enabled"); }
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
        let mut config_json: serde_json::Value = serde_json::from_str(&config.config)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let request_type = config_json
            .get("request")
            .and_then(|v| v.as_str())
            .unwrap_or("launch");

        let request = match request_type {
            "attach" => StartDebuggingRequestArgumentsRequest::Attach,
            _ => StartDebuggingRequestArgumentsRequest::Launch,
        };

        // If the configuration contains a program path with $TARGET_FRAMEWORK placeholder,
        // resolve it by reading the corresponding .csproj file to get the actual target framework.
        // Note: Zed has already expanded $ZED_WORKTREE_ROOT to the full path at this point.
        if let Some(program_value) = config_json.get_mut("program") {
            if let Some(program_str) = program_value.as_str() {
                // Check if the path contains our $TARGET_FRAMEWORK placeholder
                if program_str.contains("$TARGET_FRAMEWORK") && program_str.contains("/bin/Debug/") {
                    // Extract the worktree root
                    let worktree_root = worktree.root_path();
                    
                    // Convert absolute path to relative by removing the worktree root
                    let rel = if program_str.starts_with(&worktree_root) {
                        program_str.trim_start_matches(&worktree_root).trim_start_matches('/')
                    } else {
                        program_str
                    };
                    
                    // Parse the path: src/ConsoleApp/bin/Debug/$TARGET_FRAMEWORK/ConsoleApp.dll
                    let parts: Vec<&str> = rel.split('/').collect();
                    if let Some(bin_idx) = parts.iter().position(|p| *p == "bin") {
                        // project_dir is everything before the 'bin' segment
                        let project_dir = if bin_idx == 0 { ".".to_string() } else { parts[..bin_idx].join("/") };

                        // guess assembly name from file name
                        if let Some(file_name) = parts.last() {
                            if let Some((name, _ext)) = file_name.split_once('.') {
                                // Try to read {project_dir}/{name}.csproj
                                let csproj_path = if project_dir == "." {
                                    format!("{}.csproj", name)
                                } else {
                                    format!("{}/{}.csproj", project_dir, name)
                                };

                                if let Ok(text) = worktree.read_text_file(&csproj_path) {
                                    let proj = DotNetProject::from_csproj_text(&text, std::path::Path::new(&csproj_path));
                                    
                                    // Replace $TARGET_FRAMEWORK with the actual value
                                    let new_program = program_str.replace("$TARGET_FRAMEWORK", &proj.target_framework);
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
                configuration: serde_json::to_string(&config_json).map_err(|e| format!("Failed to serialize modified config: {e}"))?,
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

    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario, String> {
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

        // For .NET debugging, we need to know the exact DLL path.
        // Try to infer it from the build task or use a generic path.

        // Try to infer project path and name from build task
        let program = if let Some(project_arg) =
            build_task.args.iter().find(|arg| arg.ends_with(".csproj"))
        {
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
            // NOTE: The target framework placeholder will be resolved by get_dap_binary at debug time
            format!(
                "$ZED_WORKTREE_ROOT/{}/bin/Debug/$TARGET_FRAMEWORK/{}.dll",
                project_dir.replace('\\', "/"),
                project_name
            )
        } else {
            // Fallback to a generic path (will be resolved by get_dap_binary)
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
    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Cannot enumerate files in WASM sandbox"); }
    if cfg!(debug_assertions) { eprintln!(
        "[csharp_roslyn] Returning None - OmniSharp will auto-detect from working directory: {}",
        root_path
    ); }

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
