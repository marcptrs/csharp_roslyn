use zed_extension_api::{settings::LspSettings, Worktree};

/// Check if debug logging is enabled via extension setting or debug build.
pub fn is_debug_enabled(worktree: &Worktree) -> bool {
    if cfg!(debug_assertions) {
        return true;
    }

    for server_id in ["roslyn", "omnisharp", "csharp_roslyn"] {
        if let Ok(settings) = LspSettings::for_worktree(server_id, worktree) {
            if let Some(init_options) = settings.initialization_options {
                if let Some(enable_debug) = init_options.get("enable_debug_logging") {
                    if enable_debug.as_bool().unwrap_or(false) {
                        return true;
                    }
                }
            }

            if let Some(config) = settings.settings {
                if let Some(enable_debug) = config.get("enable_debug_logging") {
                    if enable_debug.as_bool().unwrap_or(false) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Debug logging macro that checks both debug_assertions and extension setting
macro_rules! debug_log {
    ($worktree:expr, $($arg:tt)*) => {
        if crate::logging::is_debug_enabled($worktree) {
            eprintln!($($arg)*);
        }
    };
}

pub(crate) use debug_log;
