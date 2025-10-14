use crate::csharp::RoslynConfig;
use std::path::{Path, PathBuf};
use zed_extension_api::{self as zed, LanguageServerId, Result};

pub struct ServerPackage {
    pub dll_path: String,
    #[allow(dead_code)]
    pub version: String,
}

const DEFAULT_VERSION: &str = "5.0.0-1.25277.114";

fn get_package_name() -> String {
    let (os, arch) = zed::current_platform();

    let platform_suffix = match (os, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "osx-arm64",
        (zed::Os::Mac, _) => "osx-x64",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "linux-arm64",
        (zed::Os::Linux, _) => "linux-x64",
        (zed::Os::Windows, _) => "win-x64",
    };

    format!("microsoft.codeanalysis.languageserver.{}", platform_suffix)
}

pub fn ensure_server(
    language_server_id: &LanguageServerId,
    config: &RoslynConfig,
    worktree: &zed::Worktree,
) -> Result<ServerPackage> {
    // Step 1: Check for server path in config (from initialization_options.serverPath)
    // Note: Skip filesystem validation in WASM - let dotnet report if path is invalid
    if let Some(ref server_path) = config.server_path {
        return Ok(ServerPackage {
            dll_path: server_path.clone(),
            version: "configured".to_string(),
        });
    }

    // Step 2: Check for manual server path in binary arguments (legacy support)
    if let Ok(lsp_settings) = zed::settings::LspSettings::for_worktree("roslyn", worktree) {
        if let Some(binary) = lsp_settings.binary {
            if let Some(server_path) = binary.arguments.as_ref().and_then(|args| args.first()) {
                return Ok(ServerPackage {
                    dll_path: server_path.clone(),
                    version: "manual".to_string(),
                });
            }
        }
    }

    // Step 3: Check cache directory
    let version = config.version.as_deref().unwrap_or(DEFAULT_VERSION);
    let cache_dir = get_cache_dir(version)?;

    if let Some(dll_path) = find_server_dll(&cache_dir) {
        return Ok(ServerPackage {
            dll_path,
            version: version.to_string(),
        });
    }

    // Step 4: Download from NuGet if not in cache
    match download_and_extract_server(language_server_id, version, &cache_dir) {
        Ok(()) => {
            if let Some(dll_path) = find_server_dll(&cache_dir) {
                Ok(ServerPackage {
                    dll_path,
                    version: version.to_string(),
                })
            } else {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(
                        "Server DLL not found after extraction".to_string(),
                    ),
                );
                Err("Server DLL not found after extraction".into())
            }
        }
        Err(e) => {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(format!(
                    "Roslyn LSP download failed ({e}). Connect to the internet and reopen the project, or set initialization_options.serverPath in settings."
                ))
            );
            Err(e)
        }
    }
}

fn get_cache_dir(version: &str) -> Result<PathBuf> {
    let cache_dir = Path::new("cache").join(version);
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {e}"))?;
    Ok(cache_dir)
}

fn download_and_extract_server(
    language_server_id: &LanguageServerId,
    version: &str,
    cache_dir: &Path,
) -> Result<()> {
    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );

    let package_name = get_package_name();
    let nupkg_url = format!(
        "https://api.nuget.org/v3-flatcontainer/{}/{}/{}.{}.nupkg",
        package_name, version, package_name, version
    );

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );

    let cache_dir_str = cache_dir.to_string_lossy().to_string();

    zed::download_file(&nupkg_url, &cache_dir_str, zed::DownloadedFileType::Zip).or_else(|e| {
        if find_server_dll(cache_dir).is_some() {
            Ok(())
        } else {
            let _ = std::fs::remove_dir_all(cache_dir);
            Err(format!("Failed to download Roslyn server: {e}"))
        }
    })
}

fn find_server_dll(cache_dir: &Path) -> Option<String> {
    let roslyn_dir = cache_dir.join(".roslyn");

    if roslyn_dir.exists() {
        if let Some(dll_path) = search_for_dll(&roslyn_dir) {
            return Some(dll_path);
        }
    }

    search_for_dll(cache_dir)
}

fn search_for_dll(dir: &Path) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == "Microsoft.CodeAnalysis.LanguageServer.dll" {
                        // Convert to absolute path
                        let absolute_path = if path.is_absolute() {
                            path
                        } else {
                            std::env::current_dir().ok()?.join(&path)
                        };
                        return Some(absolute_path.to_string_lossy().to_string());
                    }
                }
            } else if path.is_dir() {
                if let Some(result) = search_for_dll(&path) {
                    return Some(result);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_version() {
        assert!(!DEFAULT_VERSION.is_empty());
        assert!(DEFAULT_VERSION.contains('.'));
    }

    #[test]
    fn test_package_name_format() {
        let package = ServerPackage {
            dll_path: "/path/to/server.dll".to_string(),
            version: "4.10.0".to_string(),
        };
        assert!(package.dll_path.ends_with(".dll"));
        assert!(!package.version.is_empty());
    }
}
