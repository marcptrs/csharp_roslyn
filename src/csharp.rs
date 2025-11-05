use serde_json::json;
use zed_extension_api::{self as zed, LanguageServerId, Result};

pub struct CsharpRoslynExtension;

impl zed::Extension for CsharpRoslynExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Find the roslyn-wrapper binary
        let (platform, _arch) = zed::current_platform();
        let wrapper_path = find_roslyn_wrapper(platform, worktree)?;
        
        // Run roslyn-wrapper (which will handle finding/downloading Roslyn)
        Ok(zed::Command {
            command: wrapper_path,
            args: vec![],
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



/// Find solution files (.sln, .slnx, .slnf) in the workspace root
fn find_solution(worktree: &zed::Worktree) -> Option<String> {
    let root = worktree.root_path();
    let extensions = vec!["sln", "slnx", "slnf"];
    
    // Get the directory name from the root path
    let root_name = root.split('/').last().unwrap_or("");
    
    // Try variations of the directory name to handle different naming conventions
    let variations = vec![
        // Exact directory name
        root_name.to_string(),
        // PascalCase from snake_case (test_csharp_project -> TestCsharpProject)
        root_name.split('_')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        // PascalCase from kebab-case
        root_name.split('-')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        // Common names
        "Solution".to_string(),
        "solution".to_string(),
    ];
    
    // Try each variation with each extension
    for variant in &variations {
        for ext in &extensions {
            let candidate = format!("{}.{}", variant, ext);
            if worktree.read_text_file(&candidate).is_ok() {
                return Some(format!("{}/{}", root, candidate));
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

/// Find the roslyn-wrapper binary
fn find_roslyn_wrapper(platform: zed::Os, _worktree: &zed::Worktree) -> Result<String> {
    let binary_name = match platform {
        zed::Os::Windows => "roslyn-wrapper.exe",
        _ => "roslyn-wrapper",
    };
    
    // Return just the binary name
    // Zed will resolve this relative to the extension's installed directory
    // which is: ~/Library/Application Support/Zed/extensions/installed/csharp_roslyn/
    Ok(binary_name.to_string())
}

