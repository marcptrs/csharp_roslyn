use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, Command, Result, Worktree};

use crate::logging::debug_log;

const NETCOREDBG_VERSION_OFFICIAL: &str = "3.1.3-1062";
const NETCOREDBG_REPO_OFFICIAL: &str = "https://github.com/Samsung/netcoredbg";
const NETCOREDBG_VERSION_MAC_ARM64: &str = "3.1.3-1062";
const NETCOREDBG_REPO_MAC_ARM64: &str = "https://github.com/marcptrs/netcoredbg";

pub fn ensure_debugger(worktree: &Worktree) -> Result<Command> {
    let (os, arch) = zed::current_platform();
    let download_spec = get_download_spec(os, arch)?;
    let cache_dir = get_debugger_cache_dir(download_spec.version)?;

    let mut debugger_binary = find_debugger_binary(&cache_dir, os);
    if debugger_binary.is_none() {
        download_and_extract_debugger(&cache_dir, os, arch, &download_spec, worktree)?;
        debugger_binary = find_debugger_binary(&cache_dir, os);
    }

    let debugger_binary = debugger_binary.ok_or_else(|| {
        format!(
            "netcoredbg binary not found in cache directory after download: {}",
            cache_dir.display()
        )
    })?;

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

fn get_debugger_cache_dir(version: &str) -> Result<PathBuf> {
    let cache_dir = Path::new("cache").join("netcoredbg").join(version);
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create debugger cache directory: {e}"))?;
    Ok(cache_dir)
}

fn get_debugger_binary_name(os: zed::Os) -> &'static str {
    if os == zed::Os::Windows {
        "netcoredbg.exe"
    } else {
        "netcoredbg"
    }
}

struct DownloadSpec {
    repo: &'static str,
    version: &'static str,
    archive_name: &'static str,
    file_type: zed::DownloadedFileType,
}

fn get_download_spec(os: zed::Os, arch: zed::Architecture) -> Result<DownloadSpec> {
    match (os, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => Ok(DownloadSpec {
            repo: NETCOREDBG_REPO_MAC_ARM64,
            version: NETCOREDBG_VERSION_MAC_ARM64,
            archive_name: "netcoredbg-osx-arm64.tar.gz",
            file_type: zed::DownloadedFileType::GzipTar,
        }),
        (zed::Os::Mac, zed::Architecture::X8664) => Ok(DownloadSpec {
            repo: NETCOREDBG_REPO_OFFICIAL,
            version: NETCOREDBG_VERSION_OFFICIAL,
            archive_name: "netcoredbg-osx-amd64.tar.gz",
            file_type: zed::DownloadedFileType::GzipTar,
        }),
        (zed::Os::Linux, zed::Architecture::X8664) => Ok(DownloadSpec {
            repo: NETCOREDBG_REPO_OFFICIAL,
            version: NETCOREDBG_VERSION_OFFICIAL,
            archive_name: "netcoredbg-linux-amd64.tar.gz",
            file_type: zed::DownloadedFileType::GzipTar,
        }),
        (zed::Os::Linux, zed::Architecture::Aarch64) => Ok(DownloadSpec {
            repo: NETCOREDBG_REPO_OFFICIAL,
            version: NETCOREDBG_VERSION_OFFICIAL,
            archive_name: "netcoredbg-linux-arm64.tar.gz",
            file_type: zed::DownloadedFileType::GzipTar,
        }),
        (zed::Os::Windows, zed::Architecture::X8664) => Ok(DownloadSpec {
            repo: NETCOREDBG_REPO_OFFICIAL,
            version: NETCOREDBG_VERSION_OFFICIAL,
            archive_name: "netcoredbg-win64.zip",
            file_type: zed::DownloadedFileType::Zip,
        }),
        _ => Err(format!(
            "Unsupported platform for netcoredbg release: {os:?} {arch:?}"
        )),
    }
}

fn download_and_extract_debugger(
    cache_dir: &Path,
    os: zed::Os,
    _arch: zed::Architecture,
    spec: &DownloadSpec,
    worktree: &Worktree,
) -> Result<()> {
    let mut download_urls = vec![format!(
        "{}/releases/download/{}/{}",
        spec.repo, spec.version, spec.archive_name
    )];

    if !spec.version.starts_with('v') {
        download_urls.push(format!(
            "{}/releases/download/v{}/{}",
            spec.repo, spec.version, spec.archive_name
        ));
    }

    let cache_dir_str = cache_dir.to_string_lossy().to_string();
    let mut last_error: Option<String> = None;

    for download_url in download_urls {
        debug_log!(
            worktree,
            "[csharp_roslyn] Attempting to download netcoredbg from: {download_url}"
        );

        match zed::download_file(&download_url, &cache_dir_str, spec.file_type) {
            Ok(()) => {
                last_error = None;
                break;
            }
            Err(error) => {
                let message = format!("Failed to download netcoredbg from {download_url}: {error}");
                debug_log!(worktree, "[csharp_roslyn] {message}");
                last_error = Some(message);
            }
        }
    }

    if let Some(error) = last_error {
        return Err(error);
    }

    if os != zed::Os::Windows {
        if let Some(debugger_binary) = find_debugger_binary(cache_dir, os) {
            zed::make_file_executable(&debugger_binary.to_string_lossy())
                .map_err(|e| format!("Failed to make debugger executable: {e}"))?;
        }
    }

    Ok(())
}

fn find_debugger_binary(cache_dir: &Path, os: zed::Os) -> Option<PathBuf> {
    let binary_name = get_debugger_binary_name(os);

    let direct = cache_dir.join(binary_name);
    if direct.is_file() {
        return Some(direct);
    }

    if direct.is_dir() {
        let nested = direct.join(binary_name);
        if nested.is_file() {
            return Some(nested);
        }
    }

    let nested = cache_dir.join("netcoredbg").join(binary_name);
    if nested.is_file() {
        return Some(nested);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_versions() {
        assert!(NETCOREDBG_VERSION_OFFICIAL.contains('.'));
        assert!(NETCOREDBG_VERSION_MAC_ARM64.contains('.'));
    }

    #[test]
    fn test_archive_names() {
        assert_eq!(
            get_download_spec(zed::Os::Linux, zed::Architecture::X8664)
                .unwrap()
                .archive_name,
            "netcoredbg-linux-amd64.tar.gz"
        );
        assert_eq!(
            get_download_spec(zed::Os::Linux, zed::Architecture::Aarch64)
                .unwrap()
                .archive_name,
            "netcoredbg-linux-arm64.tar.gz"
        );
        assert_eq!(
            get_download_spec(zed::Os::Mac, zed::Architecture::X8664)
                .unwrap()
                .archive_name,
            "netcoredbg-osx-amd64.tar.gz"
        );
        assert_eq!(
            get_download_spec(zed::Os::Mac, zed::Architecture::Aarch64)
                .unwrap()
                .archive_name,
            "netcoredbg-osx-arm64.tar.gz"
        );
        assert_eq!(
            get_download_spec(zed::Os::Windows, zed::Architecture::X8664)
                .unwrap()
                .archive_name,
            "netcoredbg-win64.zip"
        );
    }

    #[test]
    fn test_binary_name() {
        assert_eq!(get_debugger_binary_name(zed::Os::Mac), "netcoredbg");
        assert_eq!(get_debugger_binary_name(zed::Os::Linux), "netcoredbg");
        assert_eq!(get_debugger_binary_name(zed::Os::Windows), "netcoredbg.exe");
    }
}
