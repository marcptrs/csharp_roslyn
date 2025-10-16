use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, Command, Result, Worktree};

const NETCOREDBG_VERSION: &str = "v3.1.2-1054";
const NETCOREDBG_REPO: &str = "https://github.com/marcptrs/netcoredbg";

pub fn ensure_debugger(_worktree: &Worktree) -> Result<Command> {
    let cache_dir = get_debugger_cache_dir()?;
    let debugger_binary = cache_dir
        .join("netcoredbg")
        .join(get_debugger_binary_name());

    if !debugger_binary.exists() {
        download_and_extract_debugger(&cache_dir)?;
    }

    let absolute_path = if debugger_binary.is_absolute() {
        debugger_binary
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {e}"))?
            .join(&debugger_binary)
    };

    Ok(Command {
        command: absolute_path.to_string_lossy().to_string(),
        args: vec!["--interpreter=vscode".to_string()],
        env: Default::default(),
    })
}

fn get_debugger_cache_dir() -> Result<PathBuf> {
    let cache_dir = Path::new("cache")
        .join("netcoredbg")
        .join(NETCOREDBG_VERSION);
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create debugger cache directory: {e}"))?;
    Ok(cache_dir)
}

fn get_debugger_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "netcoredbg.exe"
    } else {
        "netcoredbg"
    }
}

fn get_platform_suffix() -> Result<String> {
    let (os, arch) = zed::current_platform();

    let platform = match (os, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "osx-arm64",
        (zed::Os::Mac, _) => "osx-x64",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "linux-arm64",
        (zed::Os::Linux, zed::Architecture::X8664) => "linux-x64",
        (zed::Os::Linux, _) => "linux-arm",
        (zed::Os::Windows, _) => "win-x64",
    };

    Ok(platform.to_string())
}

fn download_and_extract_debugger(cache_dir: &Path) -> Result<()> {
    let platform = get_platform_suffix()?;
    let is_windows = cfg!(target_os = "windows");
    let (archive_name, file_type) = if is_windows {
        (
            format!("netcoredbg-{}.zip", platform),
            zed::DownloadedFileType::Zip,
        )
    } else {
        (
            format!("netcoredbg-{}.tar.gz", platform),
            zed::DownloadedFileType::GzipTar,
        )
    };
    let download_url = format!(
        "{}/releases/download/{}/{}",
        NETCOREDBG_REPO, NETCOREDBG_VERSION, archive_name
    );

    eprintln!("Attempting to download netcoredbg from: {}", download_url);

    let cache_dir_str = cache_dir.to_string_lossy().to_string();
    zed::download_file(&download_url, &cache_dir_str, file_type)
        .map_err(|e| format!("Failed to download netcoredbg from {}: {e}", download_url))?;

    let debugger_binary = cache_dir
        .join("netcoredbg")
        .join(get_debugger_binary_name());
    if debugger_binary.exists() {
        zed::make_file_executable(&debugger_binary.to_string_lossy())
            .map_err(|e| format!("Failed to make debugger executable: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_version() {
        assert!(!NETCOREDBG_VERSION.is_empty());
        assert!(NETCOREDBG_VERSION.contains('.'));
    }

    #[test]
    fn test_platform_suffix() {
        let suffix = get_platform_suffix().unwrap();
        assert!(!suffix.is_empty());
        assert_ne!(suffix, "unknown");
    }

    #[test]
    fn test_binary_name() {
        let name = get_debugger_binary_name();
        assert!(name == "netcoredbg" || name == "netcoredbg.exe");
    }
}
