use std::fs;

use zed_extension_api::Result;

pub(super) fn remove_outdated_versions(
    language_server_id: &'static str,
    version_dir: &str,
) -> Result<()> {
    let entries =
        fs::read_dir(".").map_err(|e| format!("failed to list working directory: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to load directory entry: {e}"))?;
        let Some(file_name) = entry.file_name().to_str().map(|name| name.to_string()) else {
            continue;
        };

        if file_name.starts_with(language_server_id) && file_name != version_dir {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}
