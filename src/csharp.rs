use serde_json::json;
use zed_extension_api::{self as zed, LanguageServerId, DownloadedFileType, Result};

pub struct CsharpRoslynExtension;

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Get or download roslyn-wrapper binary
        let wrapper_path = get_or_download_wrapper()?;
        
        // Get or download Roslyn LSP server
        let roslyn_lsp_path = get_or_download_roslyn_lsp()?;
        
        // Convert relative paths to absolute paths
        let wrapper_abs = to_absolute_path(&wrapper_path)?;
        let mut roslyn_lsp_abs = to_absolute_path(&roslyn_lsp_path)?;
        
        eprintln!("[csharp_roslyn] Wrapper path: {}", wrapper_abs);
        eprintln!("[csharp_roslyn] LSP path before .exe check: {}", roslyn_lsp_abs);
        
        // On Windows, check if binary needs .exe extension
        #[cfg(windows)]
        {
            if !std::fs::metadata(&roslyn_lsp_abs).is_ok_and(|m| m.is_file()) {
                eprintln!("[csharp_roslyn] Binary not found at {}, trying with .exe", roslyn_lsp_abs);
                // Try with .exe extension
                let exe_path = format!("{}.exe", roslyn_lsp_abs);
                if std::fs::metadata(&exe_path).is_ok_and(|m| m.is_file()) {
                    eprintln!("[csharp_roslyn] Found binary with .exe at {}", exe_path);
                    roslyn_lsp_abs = exe_path;
                } else {
                    eprintln!("[csharp_roslyn] Binary not found even with .exe at {}", exe_path);
                }
            } else {
                eprintln!("[csharp_roslyn] Binary found at {}", roslyn_lsp_abs);
            }
        }
        
        eprintln!("[csharp_roslyn] Final LSP path: {}", roslyn_lsp_abs);
        
        // Verify file exists before passing to wrapper
        match std::fs::metadata(&roslyn_lsp_abs) {
            Ok(metadata) => {
                eprintln!("[csharp_roslyn] File verified - Size: {} bytes", metadata.len());
            }
            Err(e) => {
                eprintln!("[csharp_roslyn] ERROR: File does not exist! Error: {}", e);
                return Err(format!("Roslyn LSP binary not found at: {}", roslyn_lsp_abs).into());
            }
        }
        
        // Pass Roslyn LSP path as argument to wrapper
        // Note: Zed will normalize the path to forward slashes in JSON serialization
        Ok(zed::Command {
            command: wrapper_abs,
            args: vec![roslyn_lsp_abs],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // Find solution file and pass it to Roslyn
        if let Some(solution_path) = find_solution(worktree) {
            let solution_uri = path_to_uri(&solution_path);
            
            // Return initialization options with solution file
            // Roslyn expects this in the initialization options
            Ok(Some(json!({
                "solution": solution_uri
            })))
        } else {
            // If no solution found, still initialize but without solution context
            // Roslyn will fall back to .csproj files in the root
            Ok(Some(json!({})))
        }
    }
}

/// Find solution files (.sln, .slnx, .slnf) in the workspace root
/// Priority: .sln > .slnx > .slnf
fn find_solution(worktree: &zed::Worktree) -> Option<String> {
    let root = worktree.root_path();
    
    // Solution file names to search for, in order of preference
    let patterns = vec![
        "*.sln",    // Traditional solution files
        "*.slnx",   // New format (VS 2022)
        "*.slnf",   // Filtered solution format
    ];

    for pattern in patterns {
        // Try all likely file names for this pattern
        // We can't list directory contents in WASM, so we try common names
        let candidates = match pattern {
            "*.sln" => vec![
                "Solution.sln",
                "solution.sln",
            ],
            "*.slnx" => vec![
                "Solution.slnx",
                "solution.slnx",
            ],
            "*.slnf" => vec![
                "Solution.slnf",
                "solution.slnf",
            ],
            _ => vec![],
        };

        for name in candidates {
            // Try to read the file to verify it exists
            if worktree.read_text_file(name).is_ok() {
                let full_path = format!("{}/{}", root, name);
                return Some(full_path);
            }
        }
    }

    None
}

/// Convert a relative path to an absolute path
/// Relative paths are relative to the extension's working directory
fn to_absolute_path(path: &str) -> Result<String> {
    let path_buf = std::path::PathBuf::from(path);
    
    // If already absolute, normalize and return
    if path_buf.is_absolute() {
        return Ok(normalize_path(&path_buf));
    }
    
    // Get current working directory and join with relative path
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    let absolute_path = cwd.join(&path_buf);
    Ok(normalize_path(&absolute_path))
}

/// Normalize path to use backslashes on Windows
fn normalize_path(path: &std::path::Path) -> String {
    let path_str = path.to_str().unwrap_or("");
    
    #[cfg(windows)]
    {
        // On Windows, convert all forward slashes to backslashes
        // This is important for process spawning which requires native path format
        path_str.replace('/', "\\")
    }
    
    #[cfg(not(windows))]
    {
        path_str.to_string()
    }
}

/// Convert file path to file:// URI
/// Handles both Windows (C:\path\to\file) and Unix (/path/to/file) paths
fn path_to_uri(path: &str) -> String {
    // Normalize backslashes to forward slashes
    let normalized = path.replace('\\', "/");
    
    // Windows paths like C:/path/to/file need to become file:///C:/path/to/file
    // Unix paths like /path/to/file need to become file:///path/to/file
    
    if normalized.len() > 1 && normalized.chars().nth(1) == Some(':') {
        // Windows absolute path (C:/, D:/, etc.)
        format!("file:///{}", normalized)
    } else if normalized.starts_with("//") {
        // UNC path (//server/share)
        format!("file:{}", normalized)
    } else if normalized.starts_with('/') {
        // Unix absolute path
        format!("file://{}", normalized)
    } else {
        // Relative path - shouldn't happen but handle it
        format!("file:///{}", normalized)
    }
}

/// Get or download the Roslyn LSP server binary from NuGet
/// Returns the absolute path to the Microsoft.CodeAnalysis.LanguageServer executable
fn get_or_download_roslyn_lsp() -> Result<String> {
    let (os, arch) = zed::current_platform();
    
    // Map to RID (Runtime Identifier) used by NuGet packages
    let (rid, binary_name) = match (os, arch) {
        (zed::Os::Windows, zed::Architecture::X8664) => ("win-x64", "Microsoft.CodeAnalysis.LanguageServer.exe"),
        (zed::Os::Windows, zed::Architecture::Aarch64) => ("win-arm64", "Microsoft.CodeAnalysis.LanguageServer.exe"),
        (zed::Os::Mac, zed::Architecture::X8664) => ("osx-x64", "Microsoft.CodeAnalysis.LanguageServer"),
        (zed::Os::Mac, zed::Architecture::Aarch64) => ("osx-arm64", "Microsoft.CodeAnalysis.LanguageServer"),
        (zed::Os::Linux, zed::Architecture::X8664) => ("linux-x64", "Microsoft.CodeAnalysis.LanguageServer"),
        (zed::Os::Linux, zed::Architecture::Aarch64) => ("linux-arm64", "Microsoft.CodeAnalysis.LanguageServer"),
        _ => return Err("Unsupported platform for Roslyn LSP".into()),
    };
    
    // Try these versions in order
    let versions = vec![
        "5.0.0-1.25277.114",
        "4.12.0",
        "4.11.0",
        "4.10.0",
    ];
    
    for version in versions {
        let package_name = format!("Microsoft.CodeAnalysis.LanguageServer.{}", rid);
        let cache_dir = format!("roslyn-lsp-{}", version);
        
        // Try to find cached binary first
        if let Ok(found_path) = find_binary_in_dir(&cache_dir, binary_name) {
            return Ok(found_path);
        }
        
        // Try to download from NuGet
        let nuget_url = format!(
            "https://www.nuget.org/api/v2/package/{}/{}",
            package_name, version
        );
        
        match zed::download_file(&nuget_url, &cache_dir, DownloadedFileType::Zip) {
            Ok(()) => {
                // After downloading, search for the binary in the extracted directory
                if let Ok(found_path) = find_binary_in_dir(&cache_dir, binary_name) {
                    // Make executable on Unix
                    if !matches!(os, zed::Os::Windows) {
                        let _ = zed::make_file_executable(&found_path);
                    }
                    return Ok(found_path);
                }
            }
            Err(_) => {
                // Continue to next version
                continue;
            }
        }
    }
    
    // If we couldn't download from NuGet, try to find global installation
    if let Ok(global_path) = find_global_roslyn_lsp() {
        return Ok(global_path);
    }
    
    Err("Failed to find or download Roslyn LSP. Please ensure you have internet access or install manually: dotnet tool install --global Microsoft.CodeAnalysis.LanguageServer".into())
}

/// Recursively search for a binary file in a directory
fn find_binary_in_dir(dir: &str, binary_name: &str) -> Result<String> {
    // Walk through directory looking for the binary
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    
                    if path.is_dir() {
                        // Recursively search subdirectories
                        if let Ok(found) = find_binary_in_dir(path.to_str().unwrap_or(dir), binary_name) {
                            return Ok(found);
                        }
                    } else if let Some(file_name) = path.file_name() {
                        if file_name.to_str() == Some(binary_name) {
                            if let Some(path_str) = path.to_str() {
                                return Ok(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Directory doesn't exist yet
        }
    }
    
    Err(format!("Binary {} not found in {}", binary_name, dir).into())
}

/// Try to find globally installed Roslyn LSP from dotnet tools
fn find_global_roslyn_lsp() -> Result<String> {
    let (os, _arch) = zed::current_platform();
    
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine home directory")?;
    
    let binary_name = match os {
        zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
        _ => "Microsoft.CodeAnalysis.LanguageServer",
    };
    
    let possible_paths = match os {
        zed::Os::Windows => vec![
            format!("{}/.dotnet/tools/{}", home, binary_name),
            format!("{}\\AppData\\Local\\Microsoft\\WinGet\\Links\\{}", home, binary_name),
        ],
        _ => vec![
            format!("{}/.dotnet/tools/{}", home, binary_name),
        ],
    };
    
    for path in possible_paths {
        if std::fs::metadata(&path).is_ok_and(|m| m.is_file()) {
            return Ok(path);
        }
    }
    
    Err("Global Roslyn LSP installation not found".into())
}

/// Get or download the roslyn-wrapper binary
/// Returns the path to the wrapper executable
fn get_or_download_wrapper() -> Result<String> {
    // Use runtime OS detection instead of compile-time cfg! checks
    // Extensions are compiled to WASM, so cfg!(windows) is always false
    let (os, arch) = zed::current_platform();
    
    let (platform, binary_name) = match (os, arch) {
        (zed::Os::Windows, _) => ("x86_64-pc-windows-msvc", "roslyn-wrapper.exe"),
        (zed::Os::Mac, zed::Architecture::Aarch64) => ("aarch64-apple-darwin", "roslyn-wrapper"),
        (zed::Os::Mac, _) => ("x86_64-apple-darwin", "roslyn-wrapper"),
        (zed::Os::Linux, _) => ("x86_64-unknown-linux-gnu", "roslyn-wrapper"),
    };
    
    // Version of roslyn-wrapper to download
    let version = "0.1.0";
    let relative_path = format!("roslyn-wrapper-{}/{}", version, binary_name);
    
    // Try to download wrapper from GitHub releases
    let download_url = format!(
        "https://github.com/marcptrs/roslyn-wrapper/releases/download/v{}/roslyn-wrapper-{}{}",
        version,
        platform,
        if matches!(os, zed::Os::Windows) { ".exe" } else { "" }
    );
    
    match zed::download_file(&download_url, &relative_path, DownloadedFileType::Uncompressed) {
        Ok(()) => {
            // Make executable on Unix
            if !matches!(os, zed::Os::Windows) {
                let _ = zed::make_file_executable(&relative_path);
            }
            
            Ok(relative_path)
        }
        Err(_) => {
            // If download fails, return the relative path anyway
            // The binary might have been manually copied there by the build script
            // or by the user for local development
            Ok(relative_path)
        }
    }
}
