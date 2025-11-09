use std::fs;
use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, Result};

const GITHUB_REPO_OWNER: &str = "marcptrs";
const GITHUB_REPO_NAME: &str = "roslyn_wrapper";

/// Get the cache directory for roslyn-wrapper
fn get_wrapper_cache_dir() -> Result<PathBuf> {
    let cache_dir = Path::new("cache").join("roslyn-wrapper");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create wrapper cache directory: {}", e))?;
    Ok(cache_dir)
}

/// Get the asset name for the current platform
fn get_platform_asset_name(platform: zed::Os, arch: zed::Architecture) -> Result<String> {
    let asset_name = match (platform, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "roslyn-wrapper-osx-arm64",
        (zed::Os::Mac, zed::Architecture::X8664) => "roslyn-wrapper-osx-x64",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "roslyn-wrapper-linux-arm64",
        (zed::Os::Linux, zed::Architecture::X8664) => "roslyn-wrapper-linux-x64",
        (zed::Os::Windows, zed::Architecture::X8664) => "roslyn-wrapper-win-x64.exe",
        _ => return Err(format!("Unsupported platform: {:?} {:?}", platform, arch)),
    };
    Ok(asset_name.to_string())
}

/// Check if a newer version is available on GitHub
fn get_latest_release_tag() -> Result<String> {
    let release = zed::latest_github_release(
        &format!("{}/{}", GITHUB_REPO_OWNER, GITHUB_REPO_NAME),
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )
    .map_err(|e| format!("Failed to fetch latest release: {}", e))?;

    Ok(release.version)
}

/// Download the wrapper binary from GitHub
fn download_wrapper_binary(download_url: &str, target_path: &Path) -> Result<()> {
    // Use Zed's download_file which handles redirects properly
    let target_path_str = target_path.to_string_lossy().to_string();
    
    zed::download_file(
        download_url,
        &target_path_str,
        zed::DownloadedFileType::Uncompressed,
    )
    .map_err(|e| format!("Failed to download wrapper from {}: {}", download_url, e))?;

    // Make the file executable using Zed's helper
    zed::make_file_executable(&target_path_str)
        .map_err(|e| format!("Failed to make wrapper executable: {}", e))?;

    Ok(())
}

/// Get the version from a version file or string
fn parse_version(version_str: &str) -> Option<semver::Version> {
    // Remove 'v' prefix if present
    let version_str = version_str.trim_start_matches('v');
    semver::Version::parse(version_str).ok()
}

/// Ensure the wrapper binary is available, downloading if necessary
pub fn ensure_wrapper(
    language_server_id: &zed::LanguageServerId,
    platform: zed::Os,
    arch: zed::Architecture,
    worktree: &zed::Worktree,
) -> Result<String> {
    let binary_name = if platform == zed::Os::Windows {
        "roslyn-wrapper.exe"
    } else {
        "roslyn-wrapper"
    };

    // First, check if wrapper is in PATH
    if let Some(path) = worktree.which(binary_name) {
        return Ok(path);
    }

    // Next, check the cache directory
    let cache_dir = get_wrapper_cache_dir()?;
    let version_file = cache_dir.join("version.txt");
    let binary_path = cache_dir.join(binary_name);

    // Report checking for updates
    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );

    // Get the latest release tag from GitHub
    let latest_tag = match get_latest_release_tag() {
        Ok(tag) => tag,
        Err(e) => {
            // If we can't reach GitHub, check if we have a cached version
            if binary_path.exists() {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                return Ok(binary_path.to_string_lossy().to_string());
            }
            let error_msg = format!(
                "Failed to get latest release info and no cached version available: {}",
                e
            );
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(error_msg.clone()),
            );
            return Err(error_msg);
        }
    };

    let latest_version = parse_version(&latest_tag);

    // Check if we have a cached version and if it's up to date
    let needs_download = if binary_path.exists() && version_file.exists() {
        match fs::read_to_string(&version_file) {
            Ok(cached_version_str) => {
                let cached_version = parse_version(&cached_version_str);
                match (cached_version, latest_version) {
                    (Some(cached), Some(latest)) => cached < latest,
                    _ => true, // If we can't parse versions, download to be safe
                }
            }
            Err(_) => true, // If we can't read version file, download
        }
    } else {
        true // Binary doesn't exist, need to download
    };

    if needs_download {
        // Report downloading status
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        let asset_name = get_platform_asset_name(platform, arch)?;
        let download_url = format!(
            "https://github.com/{}/{}/releases/download/{}/{}",
            GITHUB_REPO_OWNER, GITHUB_REPO_NAME, latest_tag, asset_name
        );

        // Download the binary
        if let Err(e) = download_wrapper_binary(&download_url, &binary_path) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(e.clone()),
            );
            return Err(e);
        }

        // Write the version file
        if let Err(e) = fs::write(&version_file, &latest_tag)
            .map_err(|e| format!("Failed to write version file: {}", e))
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(e.clone()),
            );
            return Err(e);
        }
    }

    // Clear installation status
    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::None,
    );

    Ok(binary_path.to_string_lossy().to_string())
}
