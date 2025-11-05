use serde_json::json;
use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, DownloadedFileType, Result};

pub struct CsharpRoslynExtension {
    cached_wrapper_path: Option<String>,
    cached_roslyn_path: Option<String>,
}

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self {
            cached_wrapper_path: None,
            cached_roslyn_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Get or download roslyn-wrapper binary
        let wrapper_path = self.get_or_download_wrapper(language_server_id)?;
        
        // Get or download Roslyn LSP server
        let roslyn_lsp_path = self.get_or_download_roslyn_lsp(language_server_id)?;
        
        eprintln!("[csharp_roslyn] Wrapper path: {}", wrapper_path);
        eprintln!("[csharp_roslyn] Roslyn LSP path: {}", roslyn_lsp_path);
        
        // Verify files exist
        if !fs::metadata(&roslyn_lsp_path).map_or(false, |stat| stat.is_file()) {
            return Err(format!("Roslyn LSP binary not found at: {}", roslyn_lsp_path).into());
        }
        
        if !fs::metadata(&wrapper_path).map_or(false, |stat| stat.is_file()) {
            return Err(format!("roslyn-wrapper binary not found at: {}", wrapper_path).into());
        }
        
        // Pass Roslyn LSP path as argument to wrapper
        Ok(zed::Command {
            command: wrapper_path,
            args: vec![roslyn_lsp_path],
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
            
            Ok(Some(json!({
                "solution": solution_uri
            })))
        } else {
            // Initialize without solution if not found
            Ok(Some(json!({})))
        }
    }
}

impl CsharpRoslynExtension {
    /// Get the cache directory for this extension (uses the extension's working directory)
    fn get_cache_dir(&self) -> Result<String> {
        // Extensions run in Zed's sandbox and have their working directory set to the extension's directory.
        // We use relative paths within that working directory for caching downloaded binaries.
        let cache_dir = ".cache/csharp_roslyn";
        
        // Ensure the cache directory exists
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        
        Ok(cache_dir.to_string())
    }

    /// Get or download the roslyn-wrapper binary from GitHub releases
    fn get_or_download_wrapper(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        if let Some(cached) = &self.cached_wrapper_path {
            if fs::metadata(cached).map_or(false, |stat| stat.is_file()) {
                return Ok(cached.clone());
            }
        }

        // Build absolute path to Zed cache directory for this extension
        let cache_dir = self.get_cache_dir()?;
        
        // Log for debugging
        eprintln!("[csharp_roslyn] Cache directory: {}", cache_dir);

        let (platform, arch) = zed::current_platform();
        let (platform_str, binary_name) = match (platform, arch) {
            (zed::Os::Windows, _) => ("x86_64-pc-windows-msvc", "roslyn-wrapper.exe"),
            (zed::Os::Mac, zed::Architecture::Aarch64) => ("aarch64-apple-darwin", "roslyn-wrapper"),
            (zed::Os::Mac, _) => ("x86_64-apple-darwin", "roslyn-wrapper"),
            (zed::Os::Linux, _) => ("x86_64-unknown-linux-gnu", "roslyn-wrapper"),
        };

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "marcptrs/roslyn_wrapper",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = format!("roslyn-wrapper-{}{}", platform_str, 
            if matches!(platform, zed::Os::Windows) { ".exe" } else { "" }
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching: {}", asset_name))?;

        // First check for development/testing cache (fixed path, no versioning)
        let dev_cache_path = format!("{}/roslyn-wrapper-dev/{}", cache_dir, binary_name);
        if fs::metadata(&dev_cache_path).map_or(false, |stat| stat.is_file()) {
            eprintln!("[csharp_roslyn] Found development wrapper binary at: {}", dev_cache_path);
            self.cached_wrapper_path = Some(dev_cache_path.clone());
            return Ok(dev_cache_path);
        }

        // Then check versioned cache from previous downloads
        let version_dir = format!("{}/roslyn-wrapper-{}", cache_dir, release.version);
        let binary_path = format!("{}/{}", version_dir, binary_name);

        // Check if binary already exists in versioned cache
        if fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            eprintln!("[csharp_roslyn] Found cached wrapper binary at: {}", binary_path);
            self.cached_wrapper_path = Some(binary_path.clone());
            return Ok(binary_path);
        }

        // Download from GitHub if not in cache
        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download roslyn-wrapper: {}", e))?;

            // Make executable on Unix
            if !matches!(platform, zed::Os::Windows) {
                let _ = zed::make_file_executable(&binary_path);
            }

            // Clean up old versions (keeping only the current one)
            if let Ok(entries) = fs::read_dir(&cache_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let file_name = entry.file_name();
                        let name_str = file_name.to_str().unwrap_or("");
                        if name_str.starts_with("roslyn-wrapper-") 
                            && name_str != format!("roslyn-wrapper-{}", release.version)
                            && !name_str.ends_with("-dev") {
                            let _ = fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        }

        self.cached_wrapper_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    /// Get or download the Roslyn LSP server binary from NuGet
    fn get_or_download_roslyn_lsp(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        if let Some(cached) = &self.cached_roslyn_path {
            if fs::metadata(cached).map_or(false, |stat| stat.is_file()) {
                return Ok(cached.clone());
            }
        }

        // Get absolute cache directory
        let cache_dir = self.get_cache_dir()?;

        let (platform, arch) = zed::current_platform();
        let (rid, binary_name) = match (platform, arch) {
            (zed::Os::Windows, zed::Architecture::X8664) => ("win-x64", "Microsoft.CodeAnalysis.LanguageServer.exe"),
            (zed::Os::Windows, zed::Architecture::Aarch64) => ("win-arm64", "Microsoft.CodeAnalysis.LanguageServer.exe"),
            (zed::Os::Mac, zed::Architecture::X8664) => ("osx-x64", "Microsoft.CodeAnalysis.LanguageServer"),
            (zed::Os::Mac, zed::Architecture::Aarch64) => ("osx-arm64", "Microsoft.CodeAnalysis.LanguageServer"),
            (zed::Os::Linux, zed::Architecture::X8664) => ("linux-x64", "Microsoft.CodeAnalysis.LanguageServer"),
            (zed::Os::Linux, zed::Architecture::Aarch64) => ("linux-arm64", "Microsoft.CodeAnalysis.LanguageServer"),
            _ => return Err("Unsupported platform for Roslyn LSP".into()),
        };

        // Try to download the latest version of Roslyn LSP from NuGet
        let version = "5.0.0-1.25277.114";
        let package_name = format!("Microsoft.CodeAnalysis.LanguageServer.{}", rid);
        let version_dir = format!("{}/roslyn-lsp-{}-{}", cache_dir, rid, version);

        // Try to find cached binary first
        if let Ok(found_path) = find_binary_in_dir(&version_dir, binary_name) {
            self.cached_roslyn_path = Some(found_path.clone());
            return Ok(found_path);
        }

        let nuget_url = format!(
            "https://www.nuget.org/api/v2/package/{}/{}",
            package_name, version
        );

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        let file_type = match platform {
            zed::Os::Windows => DownloadedFileType::Zip,
            _ => DownloadedFileType::GzipTar,
        };

        if zed::download_file(&nuget_url, &version_dir, file_type).is_ok() {
            if let Ok(found_path) = find_binary_in_dir(&version_dir, binary_name) {
                // Make executable on Unix
                if !matches!(platform, zed::Os::Windows) {
                    let _ = zed::make_file_executable(&found_path);
                }

                // Clean up old versions (keep current version only)
                if let Ok(entries) = fs::read_dir(&cache_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let file_name = entry.file_name();
                            let name_str = file_name.to_str().unwrap_or("");
                            if name_str.starts_with("roslyn-lsp-") 
                                && name_str != format!("roslyn-lsp-{}-{}", rid, version) {
                                let _ = fs::remove_dir_all(entry.path());
                            }
                        }
                    }
                }

                self.cached_roslyn_path = Some(found_path.clone());
                return Ok(found_path);
            }
        }

        // Try to find global installation
        if let Ok(global_path) = find_global_roslyn_lsp(platform) {
            self.cached_roslyn_path = Some(global_path.clone());
            return Ok(global_path);
        }

        Err("Failed to find or download Roslyn LSP. Install with: dotnet tool install --global Microsoft.CodeAnalysis.LanguageServer".into())
    }
}

/// Find solution files (.sln, .slnx, .slnf) in the workspace root
fn find_solution(worktree: &zed::Worktree) -> Option<String> {
    let root = worktree.root_path();
    
    let patterns = vec!["*.sln", "*.slnx", "*.slnf"];

    for pattern in patterns {
        let candidates = match pattern {
            "*.sln" => vec!["Solution.sln", "solution.sln"],
            "*.slnx" => vec!["Solution.slnx", "solution.slnx"],
            "*.slnf" => vec!["Solution.slnf", "solution.slnf"],
            _ => vec![],
        };

        for name in candidates {
            if worktree.read_text_file(name).is_ok() {
                return Some(format!("{}/{}", root, name));
            }
        }
    }

    None
}

/// Convert file path to file:// URI
fn path_to_uri(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    
    if normalized.len() > 1 && normalized.chars().nth(1) == Some(':') {
        format!("file:///{}", normalized)
    } else if normalized.starts_with("//") {
        format!("file:{}", normalized)
    } else if normalized.starts_with('/') {
        format!("file://{}", normalized)
    } else {
        format!("file:///{}", normalized)
    }
}

/// Recursively search for a binary file in a directory
fn find_binary_in_dir(dir: &str, binary_name: &str) -> Result<String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    
                    if path.is_dir() {
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
        Err(_) => {}
    }
    
    Err(format!("Binary {} not found in {}", binary_name, dir).into())
}

/// Try to find globally installed Roslyn LSP from dotnet tools
fn find_global_roslyn_lsp(platform: zed::Os) -> Result<String> {
    let binary_name = match platform {
        zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
        _ => "Microsoft.CodeAnalysis.LanguageServer",
    };
    
    // Try using the which command to find the binary on PATH first
    if let Ok(which_cmd) = std::process::Command::new("which")
        .arg(binary_name)
        .output() {
        if which_cmd.status.success() {
            let path = String::from_utf8_lossy(&which_cmd.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }
    
    // Fallback: try environment-based search if HOME is available
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let possible_paths = match platform {
            zed::Os::Windows => vec![
                format!("{}/.dotnet/tools/{}", home, binary_name),
                format!("{}\\AppData\\Local\\Microsoft\\WinGet\\Links\\{}", home, binary_name),
            ],
            _ => vec![
                format!("{}/.dotnet/tools/{}", home, binary_name),
            ],
        };
        
        for path in possible_paths {
            if fs::metadata(&path).map_or(false, |m| m.is_file()) {
                return Ok(path);
            }
        }
    }
    
    Err("Global Roslyn LSP installation not found".into())
}

