mod connection;
mod id_mapper;
mod message;
mod middleware;
mod router;

use anyhow::{Context, Result};
use middleware::{
    capability_registration::CapabilityRegistrationMiddleware,
    configuration::ConfigurationMiddleware, custom::CustomNotificationsMiddleware,
    definition_logger::DefinitionLoggerMiddleware, diagnostics::DiagnosticsMiddleware,
    document_lifecycle::DocumentLifecycleMiddleware,
    inlay_hints::InlayHintsMiddleware, initialization::InitializationMiddleware,
    project_restore::ProjectRestoreMiddleware, refresh::RefreshMiddleware, 
    solution_loader::SolutionLoaderMiddleware, MiddlewarePipeline,
};
use router::Router;
use std::env;
use std::process::Stdio;
use tokio::io;
use tokio::process::Command;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Create logs directory for proxy debugging
    let log_dir = std::path::Path::new("logs");
    std::fs::create_dir_all(&log_dir).ok();
    
    // Set up file logging for proxy
    let log_file_result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("proxy-debug.log"));
    
    match log_file_result {
        Ok(file) => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new("info"))
                )
                .with_writer(std::sync::Mutex::new(file))
                .with_ansi(false)
                .init();
        }
        Err(_) => {
            // Fallback to stderr if file creation fails
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new("info"))
                )
                .init();
        }
    }

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: roslyn-lsp-proxy <roslyn-server-path> [args...]");
        std::process::exit(1);
    }

    let server_path = &args[1];
    let server_args = if args.len() > 2 {
        &args[2..]
    } else {
        &[]
    };

    info!("Starting Roslyn LSP proxy");
    info!("Server DLL: {}", server_path);
    info!("Additional args: {:?}", server_args);

    let dotnet_path = find_dotnet().context("Failed to find dotnet executable")?;
    info!("Using dotnet at: {}", dotnet_path);

    // Use extension logs directory (platform-independent)
    let log_dir = std::path::Path::new("logs");
    std::fs::create_dir_all(&log_dir).context("Failed to create log directory")?;
    let log_dir_str = log_dir.to_string_lossy().to_string();
    
    info!("Roslyn logs will be written to: {}", log_dir_str);

    // Build command: dotnet <dll> --stdio --logLevel Information --extensionLogDirectory <log_dir> [user-args...]
    let mut command_args = vec![
        server_path.to_string(),
        "--stdio".to_string(),
        "--logLevel".to_string(),
        "Information".to_string(),
        "--extensionLogDirectory".to_string(),
        log_dir_str,
    ];
    command_args.extend(server_args.iter().map(|s| s.to_string()));

    info!("Spawning: {} {}", dotnet_path, command_args.join(" "));

    let mut server_process = Command::new(&dotnet_path)
        .args(&command_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn Roslyn server process")?;

    let server_stdin = server_process.stdin.take().context("Failed to open server stdin")?;
    let server_stdout = server_process.stdout.take().context("Failed to open server stdout")?;
    let server_stderr = server_process.stderr.take().context("Failed to open server stderr")?;
    
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let reader = tokio::io::BufReader::new(server_stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            info!("[Roslyn] {}", line);
        }
    });
    
    let client_stdin = io::stdin();
    let client_stdout = io::stdout();

    let pipeline = MiddlewarePipeline::new()
        .add(InitializationMiddleware::new())
        .add(DocumentLifecycleMiddleware::new())
        .add(SolutionLoaderMiddleware::new())
        .add(ProjectRestoreMiddleware::new())
        .add(ConfigurationMiddleware::new())
        .add(CapabilityRegistrationMiddleware::new())
        .add(DefinitionLoggerMiddleware::new())
        .add(DiagnosticsMiddleware::new())
        .add(InlayHintsMiddleware::new())
        .add(RefreshMiddleware::new())
        .add(CustomNotificationsMiddleware::new());

    let router = Router::new(
        client_stdin,
        client_stdout,
        server_stdout,
        server_stdin,
        pipeline,
    );

    info!("Proxy router started");

    if let Err(e) = router.run().await {
        error!("Router error: {}", e);
        return Err(e);
    }

    info!("Proxy shutting down");

    server_process.kill().await.ok();

    Ok(())
}

fn find_dotnet() -> Result<String> {
    // Check if dotnet is in PATH
    #[cfg(windows)]
    let which_command = "where";
    #[cfg(not(windows))]
    let which_command = "which";
    
    if let Ok(output) = std::process::Command::new(which_command).arg("dotnet").output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let path = path.lines().next().unwrap_or("").trim();
                if !path.is_empty() {
                    return Ok(path.to_string());
                }
            }
        }
    }

    // Check common locations
    #[cfg(windows)]
    let common_paths = vec![
        "C:\\Program Files\\dotnet\\dotnet.exe",
        "C:\\Program Files (x86)\\dotnet\\dotnet.exe",
    ];
    
    #[cfg(not(windows))]
    let common_paths = vec![
        "/usr/local/share/dotnet/dotnet",
        "/usr/local/bin/dotnet",
        "/usr/bin/dotnet",
        "/opt/homebrew/bin/dotnet",
    ];

    for path in common_paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    anyhow::bail!("dotnet executable not found in PATH or common locations")
}
