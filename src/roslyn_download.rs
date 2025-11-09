use std::fs;
use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, Result};

const ROSLYN_VERSION: &str = "5.0.0-1.25277.114";

/// Get the cache directory for Roslyn LSP (inside wrapper's directory)
fn get_roslyn_cache_dir() -> Result<PathBuf> {
    // Store Roslyn LSP in the wrapper's cache directory so relative paths work
    let cache_dir = Path::new("cache").join("roslyn-wrapper").join("roslyn-lsp");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create Roslyn cache directory: {}", e))?;
    Ok(cache_dir)
}

/// Get the platform-specific RID for NuGet packages
fn get_platform_rid(platform: zed::Os, arch: zed::Architecture) -> Result<String> {
    let rid = match (platform, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "osx-arm64",
        (zed::Os::Mac, zed::Architecture::X8664) => "osx-x64",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "linux-arm64",
        (zed::Os::Linux, zed::Architecture::X8664) => "linux-x64",
        (zed::Os::Windows, zed::Architecture::X8664) => "win-x64",
        (zed::Os::Windows, zed::Architecture::Aarch64) => "win-arm64",
        _ => return Err(format!("Unsupported platform: {:?} {:?}", platform, arch)),
    };
    Ok(rid.to_string())
}

/// Get the binary name for the platform
fn get_binary_name(platform: zed::Os) -> &'static str {
    if platform == zed::Os::Windows {
        "Microsoft.CodeAnalysis.LanguageServer.exe"
    } else {
        "Microsoft.CodeAnalysis.LanguageServer"
    }
}

/// Find the Roslyn binary in the extracted package directory
fn find_binary_in_dir(dir: &Path, binary_name: &str) -> Option<PathBuf> {
    // Walk the directory tree looking for the binary
    fn walk_dir(dir: &Path, binary_name: &str) -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = walk_dir(&path, binary_name) {
                        return Some(found);
                    }
                } else if path.file_name().and_then(|n| n.to_str()) == Some(binary_name) {
                    return Some(path);
                }
            }
        }
        None
    }
    
    walk_dir(dir, binary_name)
}

/// Download Roslyn LSP from nuget.org
fn download_roslyn(
    _language_server_id: &zed::LanguageServerId,
    version: &str,
    rid: &str,
    target_dir: &Path,
) -> Result<()> {
    let package_name = format!("Microsoft.CodeAnalysis.LanguageServer.{}", rid);
    
    // Use nuget.org v2 API (public, no authentication required)
    let download_url = format!(
        "https://www.nuget.org/api/v2/package/{}/{}",
        package_name, version
    );
    
    // Download and extract the .nupkg (ZIP file) directly to target_dir
    // This will create: target_dir/content/LanguageServer/{rid}/Microsoft.CodeAnalysis.LanguageServer
    zed::download_file(
        &download_url,
        &target_dir.to_string_lossy(),
        zed::DownloadedFileType::Zip,
    )
    .map_err(|e| format!("Failed to download and extract Roslyn: {}", e))?;
    
    Ok(())
}

/// Ensure Roslyn LSP is available, downloading if necessary
pub fn ensure_roslyn(
    language_server_id: &zed::LanguageServerId,
    platform: zed::Os,
    arch: zed::Architecture,
    _worktree: &zed::Worktree,
) -> Result<String> {
    let binary_name = get_binary_name(platform);
    
    // First, check if Roslyn is installed globally via dotnet tool
    let dotnet_tool_path = if platform == zed::Os::Windows {
        ".dotnet/tools/Microsoft.CodeAnalysis.LanguageServer.exe"
    } else {
        ".dotnet/tools/Microsoft.CodeAnalysis.LanguageServer"
    };
    
    if let Some(home_dir) = std::env::var("HOME").ok().or_else(|| std::env::var("USERPROFILE").ok()) {
        let global_path = Path::new(&home_dir).join(dotnet_tool_path);
        if global_path.exists() {
            return Ok(global_path.to_string_lossy().to_string());
        }
    }
    
    // Check the cache directory
    let cache_dir = get_roslyn_cache_dir()?;
    let version_dir = cache_dir.join(ROSLYN_VERSION);
    let version_file = cache_dir.join("version.txt");
    
    // Check if we already have this version cached
    let needs_download = if version_dir.exists() && version_file.exists() {
        match fs::read_to_string(&version_file) {
            Ok(cached_version) => cached_version.trim() != ROSLYN_VERSION,
            Err(_) => true,
        }
    } else {
        true
    };
    
    if needs_download {
        // Report downloading status
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        
        // Clean up old version if it exists
        if version_dir.exists() {
            let _ = fs::remove_dir_all(&version_dir);
        }
        
        fs::create_dir_all(&version_dir)
            .map_err(|e| format!("Failed to create version directory: {}", e))?;
        
        let rid = get_platform_rid(platform, arch)?;
        
        if let Err(e) = download_roslyn(language_server_id, ROSLYN_VERSION, &rid, &version_dir) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(e.clone()),
            );
            return Err(e);
        }
        
        // Write the version file
        if let Err(e) = fs::write(&version_file, ROSLYN_VERSION)
            .map_err(|e| format!("Failed to write version file: {}", e))
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(e.clone()),
            );
            return Err(e);
        }
    }
    
    // Find the binary in the version directory
    if let Some(binary_path) = find_binary_in_dir(&version_dir, binary_name) {
        // Make it executable on Unix platforms
        if platform != zed::Os::Windows {
            let _ = zed::make_file_executable(&binary_path.to_string_lossy());
        }
        
        // Clear installation status
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );
        
        // Strip the cache/roslyn-wrapper/ prefix to make it relative to wrapper's directory
        // The path is: cache/roslyn-wrapper/roslyn-lsp/.../binary
        // We want: roslyn-lsp/.../binary
        let path_str = binary_path.to_string_lossy();
        let wrapper_prefix = "cache/roslyn-wrapper/";
        let relative_path = if path_str.starts_with(wrapper_prefix) {
            path_str.strip_prefix(wrapper_prefix).unwrap()
        } else {
            // Fallback to full path if prefix doesn't match
            path_str.as_ref()
        };
        
        return Ok(relative_path.to_string());
    }
    
    let error_msg = format!("Roslyn binary not found after extraction in {}", version_dir.display());
    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Failed(error_msg.clone()),
    );
    Err(error_msg)
}
