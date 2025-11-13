use std::fs;
use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, Result};

const OMNISHARP_VERSION: &str = "1.39.14";
const GITHUB_REPO_OWNER: &str = "OmniSharp";
const GITHUB_REPO_NAME: &str = "omnisharp-roslyn";

/// Get the cache directory for OmniSharp-Roslyn
fn get_omnisharp_cache_dir() -> Result<PathBuf> {
    let cache_dir = Path::new("cache").join("omnisharp-roslyn");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create OmniSharp cache directory: {}", e))?;
    Ok(cache_dir)
}

/// Get the asset name for the current platform
fn get_platform_asset_name(platform: zed::Os, arch: zed::Architecture) -> Result<String> {
    let asset_name = match (platform, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "omnisharp-osx-arm64-net6.0.tar.gz",
        (zed::Os::Mac, zed::Architecture::X8664) => "omnisharp-osx-x64-net6.0.tar.gz",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "omnisharp-linux-arm64-net6.0.tar.gz",
        (zed::Os::Linux, zed::Architecture::X8664) => "omnisharp-linux-x64-net6.0.tar.gz",
        (zed::Os::Windows, zed::Architecture::X8664) => "omnisharp-win-x64-net6.0.zip",
        (zed::Os::Windows, zed::Architecture::Aarch64) => "omnisharp-win-arm64-net6.0.zip",
        _ => return Err(format!("Unsupported platform: {:?} {:?}", platform, arch)),
    };
    Ok(asset_name.to_string())
}

/// Get the binary name for the platform
fn get_binary_name(platform: zed::Os) -> &'static str {
    if platform == zed::Os::Windows {
        "OmniSharp.exe"
    } else {
        "OmniSharp"
    }
}

/// Get the version from a version file or string
fn parse_version(version_str: &str) -> Option<semver::Version> {
    // Remove 'v' prefix if present
    let version_str = version_str.trim_start_matches('v');
    semver::Version::parse(version_str).ok()
}

/// Download OmniSharp-Roslyn from GitHub releases
fn download_omnisharp(
    version: &str,
    asset_name: &str,
    target_dir: &Path,
    platform: zed::Os,
) -> Result<()> {
    let download_url = format!(
        "https://github.com/{}/{}/releases/download/v{}/{}",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME, version, asset_name
    );

    let file_type = if platform == zed::Os::Windows {
        zed::DownloadedFileType::Zip
    } else {
        zed::DownloadedFileType::GzipTar
    };

    zed::download_file(&download_url, &target_dir.to_string_lossy(), file_type)
        .map_err(|e| format!("Failed to download and extract OmniSharp: {}", e))?;

    Ok(())
}

/// Ensure OmniSharp-Roslyn is available, downloading if necessary
pub fn ensure_omnisharp(
    language_server_id: &zed::LanguageServerId,
    platform: zed::Os,
    arch: zed::Architecture,
    worktree: &zed::Worktree,
) -> Result<String> {
    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] ensure_omnisharp called"); }
    let binary_name = get_binary_name(platform);
    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Binary name: {}", binary_name); }

    // First, check if OmniSharp is in PATH
    if let Some(path) = worktree.which(binary_name) {
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Found OmniSharp in PATH: {}", path); }
        return Ok(path);
    }

    // Check the cache directory
    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] OmniSharp not in PATH, checking cache"); }
    let cache_dir = get_omnisharp_cache_dir()?;
    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Cache dir: {:?}", cache_dir); }
    let version_dir = cache_dir.join(OMNISHARP_VERSION);
    let version_file = cache_dir.join("version.txt");
    let binary_path = version_dir.join(binary_name);
    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Binary path: {:?}", binary_path); }

    // Check if we already have this version cached
    let needs_download = if version_dir.exists() && version_file.exists() {
        match fs::read_to_string(&version_file) {
            Ok(cached_version) => {
                let cached = parse_version(&cached_version);
                let current = parse_version(OMNISHARP_VERSION);
                match (cached, current) {
                    (Some(c), Some(cur)) => c < cur,
                    _ => cached_version.trim() != OMNISHARP_VERSION,
                }
            }
            Err(_) => true,
        }
    } else {
        true
    };

    if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Needs download: {}", needs_download); }

    if needs_download {
        // Report downloading status
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Starting download"); }
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        // Clean up old version if it exists
        if version_dir.exists() {
            if cfg!(debug_assertions) {             if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Cleaning up old version"); } }
            let _ = fs::remove_dir_all(&version_dir);
        }

        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Creating version directory"); }
        fs::create_dir_all(&version_dir)
            .map_err(|e| format!("Failed to create version directory: {}", e))?;

        let asset_name = get_platform_asset_name(platform, arch)?;
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Asset name: {}", asset_name); }

        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Downloading OmniSharp"); }
        if let Err(e) = download_omnisharp(OMNISHARP_VERSION, &asset_name, &version_dir, platform) {
            if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Download failed: {}", e); }
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(e.clone()),
            );
            return Err(e);
        }
        if cfg!(debug_assertions) { eprintln!("[csharp_roslyn] Download completed"); }

        // Make the binary executable on Unix platforms
        if platform != zed::Os::Windows {
            let _ = zed::make_file_executable(&binary_path.to_string_lossy());
        }

        // Write the version file
        if let Err(e) = fs::write(&version_file, OMNISHARP_VERSION)
            .map_err(|e| format!("Failed to write version file: {}", e))
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(e.clone()),
            );
            return Err(e);
        }
    }

    // Verify binary exists
    if !binary_path.exists() {
        let error_msg = format!("OmniSharp binary not found at {}", binary_path.display());
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Failed(error_msg.clone()),
        );
        return Err(error_msg);
    }

    // Clear installation status
    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::None,
    );

    Ok(binary_path.to_string_lossy().to_string())
}
