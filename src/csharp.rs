use crate::nuget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, Result};

#[cfg(all(target_family = "wasm", target_arch = "wasm32"))]
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoslynConfig {
    pub dotnet_sdk_path: Option<String>,
    pub version: Option<String>,
    pub server_path: Option<String>,
    pub server_args: Vec<String>,
    pub inlay_hints: InlayHintsConfig,
    pub semantic_tokens: SemanticTokensConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlayHintsConfig {
    pub enabled: bool,
    pub parameter_names: bool,
    pub type_hints: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokensConfig {
    pub enabled: bool,
}

impl Default for RoslynConfig {
    fn default() -> Self {
        Self {
            dotnet_sdk_path: None,
            version: None,
            server_path: None,
            server_args: Vec::new(),
            inlay_hints: InlayHintsConfig {
                enabled: true,
                parameter_names: true,
                type_hints: true,
            },
            semantic_tokens: SemanticTokensConfig { enabled: true },
        }
    }
}

pub fn load_config(worktree: &zed::Worktree) -> RoslynConfig {
    let mut config = RoslynConfig::default();

    if let Ok(lsp_settings) = zed::settings::LspSettings::for_worktree("roslyn", worktree) {
        if let Some(binary) = lsp_settings.binary {
            if let Some(path) = binary.path {
                config.dotnet_sdk_path = Some(path);
            }
            if let Some(args) = binary.arguments {
                if !args.is_empty() {
                    if args[0].ends_with(".dll") {
                        config.server_path = Some(args[0].clone());
                        config.server_args = args[1..].to_vec();
                    } else {
                        config.server_args = args;
                    }
                }
            }
        }

        if let Some(init_options) = lsp_settings.initialization_options {
            apply_initialization_options(&mut config, init_options);
        }
    }

    config
}

fn apply_initialization_options(config: &mut RoslynConfig, options: Value) {
    if let Value::Object(map) = options {
        if let Some(Value::Bool(enabled)) = map.get("semanticTokens") {
            config.semantic_tokens.enabled = *enabled;
        }

        if let Some(inlay_hints) = map.get("inlayHints") {
            match inlay_hints {
                Value::Bool(enabled) => {
                    config.inlay_hints.enabled = *enabled;
                }
                Value::Object(hints_map) => {
                    if let Some(Value::Bool(enabled)) = hints_map.get("enabled") {
                        config.inlay_hints.enabled = *enabled;
                    }
                    if let Some(Value::Bool(param_names)) = hints_map.get("parameterNames") {
                        config.inlay_hints.parameter_names = *param_names;
                    }
                    if let Some(Value::Bool(type_hints)) = hints_map.get("typeHints") {
                        config.inlay_hints.type_hints = *type_hints;
                    }
                }
                _ => {}
            }
        }

        if let Some(Value::String(version)) = map.get("version") {
            config.version = Some(version.clone());
        }

        if let Some(Value::String(server_path)) = map.get("serverPath") {
            config.server_path = Some(server_path.clone());
        }
    }
}

#[cfg(all(target_family = "wasm", target_arch = "wasm32"))]
const PROXY_BINARY: &[u8] = {
    const BYTES: &[u8] = include_bytes!(concat!("../", env!("PROXY_BINARY_PATH")));
    const _: () = assert!(
        BYTES.len() > 1000000,
        "Proxy binary seems too small - build may have failed"
    );
    BYTES
};

#[cfg(all(target_family = "wasm", target_arch = "wasm32"))]
fn ensure_proxy() -> Result<PathBuf> {
    let proxy_filename = if cfg!(target_os = "windows") {
        "roslyn-lsp-proxy.exe"
    } else {
        "roslyn-lsp-proxy"
    };

    let proxy_dir = PathBuf::from("bin");
    let proxy_path = proxy_dir.join(proxy_filename);

    fs::create_dir_all(&proxy_dir)
        .map_err(|e| format!("Failed to create proxy directory: {}", e))?;

    if PROXY_BINARY.is_empty() {
        return Err("Embedded proxy binary is empty - build may have failed".to_string());
    }

    fs::write(&proxy_path, PROXY_BINARY).map_err(|e| {
        format!(
            "Failed to write proxy binary ({} bytes): {}",
            PROXY_BINARY.len(),
            e
        )
    })?;

    #[cfg(not(target_os = "windows"))]
    {
        zed::make_file_executable(&proxy_path.to_string_lossy())
            .map_err(|e| format!("Failed to set proxy permissions: {}", e))?;
    }

    Ok(proxy_path)
}

fn find_solution_file(worktree: &zed::Worktree) -> Option<String> {
    let workspace_root = worktree.root_path();

    // Helper to join paths properly for both Windows and Unix
    let join_path = |base: &str, parts: &[&str]| -> String {
        let mut path = PathBuf::from(base);
        for part in parts {
            path.push(part);
        }
        path.to_string_lossy().to_string()
    };

    // Extract just the directory name from the workspace root path
    let dir_name = workspace_root
        .trim_end_matches('/')
        .trim_end_matches('\\')
        .split(&['/', '\\'][..])
        .next_back()
        .unwrap_or("");
    
    // Try directory name-based solution with case variations
    if !dir_name.is_empty() {
        let variants = vec![
            dir_name.to_string(),
            dir_name.to_lowercase(),
            {
                let mut chars = dir_name.chars();
                if let Some(first) = chars.next() {
                    format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase())
                } else {
                    String::new()
                }
            },
            dir_name.to_uppercase(),
        ];
        
        for variant in variants {
            if variant.is_empty() {
                continue;
            }
            let candidate = format!("{}.sln", variant);
            if worktree.read_text_file(&candidate).is_ok() {
                return Some(join_path(&workspace_root, &[&candidate]));
            }
        }
    }

    // Try common solution file names
    for name in &["solution.sln", "Solution.sln"] {
        if worktree.read_text_file(name).is_ok() {
            return Some(join_path(&workspace_root, &[name]));
        }
    }

    None
}

pub fn get_initialization_options(worktree: &zed::Worktree) -> Result<Option<Value>> {
    let mut options = serde_json::Map::new();

    if let Ok(lsp_settings) = zed::settings::LspSettings::for_worktree("roslyn", worktree) {
        if let Some(Value::Object(user_options)) = lsp_settings.initialization_options {
            for (key, value) in user_options {
                options.insert(key, value);
            }
        }
    }

    if !options.contains_key("enableImportCompletion") {
        options.insert("enableImportCompletion".to_string(), Value::Bool(true));
    }

    if !options.contains_key("inlayHints") {
        options.insert("inlayHints".to_string(), Value::Bool(true));
    }
    if !options.contains_key("enableAnalyzersSupport") {
        options.insert("enableAnalyzersSupport".to_string(), Value::Bool(false));
    }
    if !options.contains_key("organizeImportsOnFormat") {
        options.insert("organizeImportsOnFormat".to_string(), Value::Bool(false));
    }
    if !options.contains_key("enableDecompilationSupport") {
        options.insert("enableDecompilationSupport".to_string(), Value::Bool(true));
    }
    if !options.contains_key("enableEditorConfigSupport") {
        options.insert("enableEditorConfigSupport".to_string(), Value::Bool(false));
    }

    if let Some(solution_path) = find_solution_file(worktree) {
        // Convert to proper file:// URI
        let solution_uri = if solution_path.starts_with('/') {
            format!("file://{}", solution_path)
        } else if solution_path.contains(":\\") || solution_path.contains(":/") {
            let normalized = solution_path.replace('\\', "/");
            format!("file:///{}", normalized)
        } else {
            format!("file://{}", solution_path)
        };
        
        options.insert("solution".to_string(), Value::String(solution_uri));
    }
    Ok(Some(Value::Object(options)))
}

pub fn get_extra_args(worktree: &zed::Worktree) -> Vec<String> {
    let mut extra_args = Vec::new();

    extra_args.push("--telemetryLevel".to_string());
    extra_args.push("None".to_string());

    if let Ok(lsp_settings) = zed::settings::LspSettings::for_worktree("roslyn", worktree) {
        if let Some(binary) = lsp_settings.binary {
            if let Some(args) = binary.arguments {
                if args.len() > 1 {
                    extra_args.extend(args[1..].to_vec());
                }
            }
        }
    }

    extra_args
}

pub fn get_environment_variables(worktree: &zed::Worktree) -> Option<HashMap<String, String>> {
    let mut env_vars = HashMap::new();
    let mut has_vars = false;

    if let Ok(lsp_settings) = zed::settings::LspSettings::for_worktree("roslyn", worktree) {
        if let Some(Value::Object(options)) = lsp_settings.initialization_options {
            if let Some(Value::Object(trace_options)) = options.get("trace") {
                if let Some(Value::Bool(enabled)) = trace_options.get("protocol") {
                    if *enabled {
                        env_vars.insert("ROSLYN_LSP_TRACE".to_string(), "verbose".to_string());
                        has_vars = true;
                    }
                }
            }
        }
    }

    if has_vars {
        Some(env_vars)
    } else {
        None
    }
}

pub struct CsharpRoslynExtension;

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let config = load_config(worktree);

        let server_package = nuget::ensure_server(language_server_id, &config, worktree)?;

        #[cfg(all(target_family = "wasm", target_arch = "wasm32"))]
        let proxy_path = ensure_proxy()?;

        #[cfg(not(all(target_family = "wasm", target_arch = "wasm32")))]
        let proxy_path = PathBuf::from("roslyn-lsp-proxy");

        let extra_args = get_extra_args(worktree);
        let env_vars = get_environment_variables(worktree);

        let mut args = vec![server_package.dll_path.clone()];
        args.extend(extra_args);

        let env = env_vars
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<(String, String)>>();

        let proxy_path_str = proxy_path.to_string_lossy().to_string();

        let command = zed::Command {
            command: proxy_path_str.clone(),
            args,
            env,
        };

        Ok(command)
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let mut options = get_initialization_options(worktree)?;

        if let Some(ref mut value) = options {
            if let Some(object) = value.as_object_mut() {
                let config = load_config(worktree);

                if config.semantic_tokens.enabled {
                    object.insert("semanticTokens".to_string(), Value::Bool(true));

                    let token_types = vec![
                        "class",
                        "interface",
                        "struct",
                        "enum",
                        "delegate",
                        "record",
                        "method",
                        "extensionMethod",
                        "constructor",
                        "operator",
                        "property",
                        "event",
                        "field",
                        "constant",
                        "parameter",
                        "variable",
                        "localFunction",
                        "typeParameter",
                        "keyword",
                        "controlKeyword",
                        "string",
                        "number",
                        "comment",
                        "documentation",
                        "namespace",
                        "label",
                        "preprocessorKeyword",
                        "preprocessorText",
                        "excludedCode",
                        "attributeClass",
                        "enumMember",
                        "staticSymbol",
                        "overriddenSymbol",
                        "abstractSymbol",
                        "deprecatedSymbol",
                    ];

                    let token_modifiers = vec![
                        "static",
                        "abstract",
                        "virtual",
                        "override",
                        "sealed",
                        "readonly",
                        "const",
                        "async",
                        "deprecated",
                        "defaultLibrary",
                        "definition",
                        "documentation",
                    ];

                    let semantic_config = zed::serde_json::json!({
                        "enabled": true,
                        "tokenTypes": token_types,
                        "tokenModifiers": token_modifiers
                    });

                    object.insert("semanticTokensConfig".to_string(), semantic_config);
                }

                if config.inlay_hints.enabled {
                    let inlay_hints_config = zed::serde_json::json!({
                        "enabled": true,
                        "parameterNames": config.inlay_hints.parameter_names,
                        "typeHints": config.inlay_hints.type_hints
                    });

                    object.insert("inlayHints".to_string(), inlay_hints_config);
                }

                let diagnostic_config = zed::serde_json::json!({
                    "enabled": true,
                    "enableCompilationErrors": true,
                    "enableSemanticErrors": true,
                    "enableNullableWarnings": true,
                    "enableDeprecationWarnings": true,
                    "enableCodeAnalysis": true,
                    "errorCodes": {
                        "cs0103": "error",
                        "cs0246": "error",
                        "cs0117": "error",
                        "cs8602": "warning",
                        "cs8600": "warning",
                        "cs8604": "warning",
                        "cs8618": "warning",
                        "cs0162": "information",
                        "cs0168": "information",
                        "cs0219": "information",
                        "cs0414": "information",
                        "cs0618": "warning",
                        "cs0612": "warning"
                    }
                });

                object.insert("diagnosticsConfig".to_string(), diagnostic_config);
            }
        }

        Ok(options)
    }

    fn get_dap_binary(
        &mut self,
        _adapter_name: String,
        config: zed::DebugTaskDefinition,
        _user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<zed::DebugAdapterBinary, String> {
        let workspace_folder = worktree.root_path();

        let command = crate::debugger::ensure_debugger(worktree)
            .map_err(|e| format!("Failed to get debugger: {e}"))?;

        let mut raw_json: Value = zed::serde_json::from_str(&config.config)
            .map_err(|e| format!("Failed to parse debug configuration: {e}"))?;
        let mut config_json = if let Some(inner) = raw_json.get_mut("config") {
            inner.take()
        } else {
            raw_json
        };

        if let Some(obj) = config_json.as_object_mut() {
            for (_key, value) in obj.iter_mut() {
                if let Some(s) = value.as_str() {
                    let expanded = s.replace("${workspaceFolder}", &workspace_folder);
                    *value = Value::String(expanded);
                }
            }
        }

        let request_kind = match config_json.get("request") {
            Some(launch) if launch == "launch" => {
                zed::StartDebuggingRequestArgumentsRequest::Launch
            }
            Some(attach) if attach == "attach" => {
                zed::StartDebuggingRequestArgumentsRequest::Attach
            }
            _ => zed::StartDebuggingRequestArgumentsRequest::Launch,
        };

        let config_str = zed::serde_json::to_string(&config_json)
            .map_err(|e| format!("Failed to serialize debug configuration: {e}"))?;

        Ok(zed::DebugAdapterBinary {
            command: Some(command.command),
            arguments: command.args,
            cwd: Some(worktree.root_path()),
            envs: command.env,
            request_args: zed::StartDebuggingRequestArguments {
                request: request_kind,
                configuration: config_str,
            },
            connection: None,
        })
    }

    fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        config: zed::serde_json::Value,
    ) -> Result<zed::StartDebuggingRequestArgumentsRequest, String> {
        if config.is_null() {
            return Err("Config is null - awaiting locator resolution".to_string());
        }

        let cfg = if let Some(inner) = config.get("config") {
            inner
        } else {
            &config
        };
        match cfg.get("request") {
            Some(launch) if launch == "launch" => {
                Ok(zed::StartDebuggingRequestArgumentsRequest::Launch)
            }
            Some(attach) if attach == "attach" => {
                Ok(zed::StartDebuggingRequestArgumentsRequest::Attach)
            }
            Some(value) => Err(format!(
                "Unexpected value for `request` key in C# debug adapter configuration: {value:?}"
            )),
            None => Err("Missing `request` field in debug configuration".to_string()),
        }
    }

    fn dap_config_to_scenario(
        &mut self,
        config: zed::DebugConfig,
    ) -> Result<zed::DebugScenario, String> {
        let (program, cwd, args, envs) = match config.request {
            zed::DebugRequest::Launch(ref launch) => {
                let program = launch.program.clone();
                let cwd = launch.cwd.clone().unwrap_or_else(|| ".".to_string());
                let args = launch.args.clone();
                let envs = launch.envs.clone();
                (program, cwd, args, envs)
            }
            zed::DebugRequest::Attach(_) => {
                return Err("Attach is not supported via dap_config_to_scenario".to_string());
            }
        };

        let mut debug_config = serde_json::Map::new();
        debug_config.insert("type".to_string(), Value::String("netcoredbg".to_string()));
        debug_config.insert("request".to_string(), Value::String("launch".to_string()));
        debug_config.insert("program".to_string(), Value::String(program.clone()));
        debug_config.insert("cwd".to_string(), Value::String(cwd.clone()));

        if !args.is_empty() {
            debug_config.insert(
                "args".to_string(),
                Value::Array(args.iter().map(|a| Value::String(a.clone())).collect()),
            );
        }

        if !envs.is_empty() {
            let env_obj: serde_json::Map<String, Value> = envs
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            debug_config.insert("env".to_string(), Value::Object(env_obj));
        }

        let stop_at_entry = config.stop_on_entry.unwrap_or(false);
        debug_config.insert("stopAtEntry".to_string(), Value::Bool(stop_at_entry));
        debug_config.insert(
            "console".to_string(),
            Value::String("integratedTerminal".to_string()),
        );

        let config_str = zed::serde_json::to_string(&debug_config)
            .map_err(|e| format!("Failed to serialize debug configuration: {e}"))?;

        Ok(zed::DebugScenario {
            label: format!("Debug {}", program.split('/').next_back().unwrap_or(&program)),
            adapter: config.adapter,
            build: None,
            config: config_str,
            tcp_connection: None,
        })
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: zed::TaskTemplate,
        resolved_label: String,
        _debug_adapter_name: String,
    ) -> Option<zed::DebugScenario> {
        let cmd = &build_task.command;
        {
            let cmd_name = Path::new(cmd)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(cmd);
            let is_dotnet = cmd_name == "dotnet" || cmd_name == "dotnet.exe";
            if !is_dotnet {
                return None;
            }
        }

        let collect_program_args = |args: &Vec<String>| -> Vec<String> {
            if let Some(idx) = args.iter().position(|a| a == "--") {
                args[idx + 1..].to_vec()
            } else {
                Vec::new()
            }
        };

        let args = build_task.args.clone();
        if args.is_empty() {
            return None;
        }

        let program_args = collect_program_args(&args);

        let derived_build_task = match args.first().map(|s| s.as_str()) {
            Some("run") => {
                let mut derived = build_task.clone();
                let mut new_args = vec!["build".to_string()];

                let mut iter = args.iter().skip(1);
                while let Some(arg) = iter.next() {
                    if arg == "--" {
                        break;
                    } else if arg == "--project" {
                        if let Some(project_file) = iter.next() {
                            new_args.push(project_file.clone());
                        }
                    } else if !arg.starts_with("--") || arg == "--configuration" || arg == "-c" {
                        new_args.push(arg.clone());
                        if arg == "--configuration" || arg == "-c" {
                            if let Some(val) = iter.next() {
                                new_args.push(val.clone());
                            }
                        }
                    }
                }

                derived.args = new_args;
                derived
            }
            _ => {
                return None;
            }
        };

        let mut derived_build_task = derived_build_task;
        let mut env = derived_build_task.env.clone();
        if !program_args.is_empty() {
            env.push((
                "ZED_DOTNET_PROGRAM_ARGS".to_string(),
                serde_json::to_string(&program_args).unwrap_or_default(),
            ));
        }
        derived_build_task.env = env;

        Some(zed::DebugScenario {
            label: format!("Debug {}", resolved_label),
            adapter: "netcoredbg".to_string(),
            build: Some(zed::BuildTaskDefinition::Template(
                zed::BuildTaskDefinitionTemplatePayload {
                    template: derived_build_task.clone(),
                    locator_name: Some(locator_name.clone()),
                },
            )),
            config: "null".to_string(),
            tcp_connection: None,
        })
    }

    fn run_dap_locator(
        &mut self,
        _locator_name: String,
        build_task: zed::TaskTemplate,
    ) -> Result<zed::DebugRequest, String> {
        let cwd_str = build_task
            .cwd
            .as_ref()
            .ok_or_else(|| "Build task must have a cwd".to_string())?;

        let mut configuration = String::from("Debug");
        let mut args_iter = build_task.args.iter().peekable();
        while let Some(arg) = args_iter.next() {
            if arg == "--configuration" || arg == "-c" {
                if let Some(val) = args_iter.next() {
                    configuration = val.clone();
                }
            }
        }

        let mut project_name: Option<String> = None;
        let mut project_dir: Option<String> = None;
        let mut iter = build_task.args.iter();
        while let Some(arg) = iter.next() {
            if arg == "--project" {
                if let Some(path) = iter.next() {
                    let path_clean = path.replace("${workspaceFolder}", cwd_str);
                    if let Some(name) = path_clean
                        .rsplit('/')
                        .next()
                        .and_then(|n| n.strip_suffix(".csproj"))
                    {
                        project_name = Some(name.to_string());
                    }
                    if let Some((dir, _)) = path_clean.rsplit_once('/') {
                        project_dir = Some(dir.to_string());
                    } else {
                        project_dir = Some(cwd_str.to_string());
                    }
                }
                break;
            } else if arg.ends_with(".csproj") {
                let path_clean = arg.replace("${workspaceFolder}", cwd_str);
                if let Some(name) = path_clean
                    .rsplit('/')
                    .next()
                    .and_then(|n| n.strip_suffix(".csproj"))
                {
                    project_name = Some(name.to_string());
                }
                if let Some((dir, _)) = path_clean.rsplit_once('/') {
                    project_dir = Some(dir.to_string());
                } else {
                    project_dir = Some(cwd_str.to_string());
                }
                break;
            }
        }

        let proj_name = project_name
            .ok_or_else(|| "Could not determine project name from build task args".to_string())?;

        let proj_dir = project_dir.unwrap_or_else(|| cwd_str.to_string());

        // Find the DLL using platform-specific search
        let dll_path = {
            #[cfg(target_os = "windows")]
            {
                // On Windows, use PowerShell's Get-ChildItem (dir)
                let find_output = zed::process::Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-NonInteractive")
                    .arg("-Command")
                    .arg(format!(
                        "Get-ChildItem -Path '{}/bin/{}' -Filter '{}.dll' -Recurse -File | Select-Object -First 1 -ExpandProperty FullName",
                        proj_dir, configuration, proj_name
                    ))
                    .output();

                match find_output {
                    Ok(output) => {
                        if output.status != Some(0) {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            return Err(format!(
                                "Could not locate DLL: PowerShell command failed: {}",
                                stderr
                            ));
                        }

                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let dll = stdout
                            .lines()
                            .next()
                            .ok_or_else(|| {
                                format!(
                                    "No DLL found for project '{}' in {}/bin/{}",
                                    proj_name, proj_dir, configuration
                                )
                            })?
                            .trim()
                            .to_string();

                        dll
                    }
                    Err(e) => {
                        return Err(format!("Failed to search for DLL: {}", e));
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                // On Unix-like systems, use find
                let find_output = zed::process::Command::new("find")
                    .arg(format!("{}/bin/{}", proj_dir, configuration))
                    .arg("-name")
                    .arg(format!("{}.dll", proj_name))
                    .arg("-type")
                    .arg("f")
                    .output();

                match find_output {
                    Ok(output) => {
                        if output.status != Some(0) {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            return Err(format!(
                                "Could not locate DLL: find command failed: {}",
                                stderr
                            ));
                        }

                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let dll = stdout
                            .lines()
                            .next()
                            .ok_or_else(|| {
                                format!(
                                    "No DLL found for project '{}' in {}/bin/{}",
                                    proj_name, proj_dir, configuration
                                )
                            })?
                            .trim()
                            .to_string();

                        dll
                    }
                    Err(e) => {
                        return Err(format!("Failed to search for DLL: {}", e));
                    }
                }
            }
        };

        let mut args: Vec<String> = Vec::new();
        let mut envs = build_task.env.clone();
        if let Some((idx, (_, val))) = envs
            .iter()
            .enumerate()
            .find(|(_, (k, _))| k == "ZED_DOTNET_PROGRAM_ARGS")
        {
            if let Ok(restored) = serde_json::from_str::<Vec<String>>(val) {
                args = restored;
            }
            envs.remove(idx);
        }

        let request = zed::DebugRequest::Launch(zed::LaunchRequest {
            program: dll_path,
            cwd: Some(cwd_str.to_string()),
            args: args.clone(),
            envs: envs.clone(),
        });

        Ok(request)
    }
}
