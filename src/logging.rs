use zed_extension_api::{settings::LspSettings, Worktree};

/// Check if debug logging is enabled via extension setting or debug build
pub fn is_debug_enabled(worktree: &Worktree) -> bool {
    // Always enable in debug builds
    if cfg!(debug_assertions) {
        return true;
    }
    
    // Check extension setting via LSP settings
    if let Ok(settings) = LspSettings::for_worktree("csharp_roslyn", worktree) {
        if let Some(init_options) = settings.initialization_options {
            if let Some(enable_debug) = init_options.get("enable_debug_logging") {
                return enable_debug.as_bool().unwrap_or(false);
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