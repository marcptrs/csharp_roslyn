use std::path::{Path, PathBuf};
use zed_extension_api as zed;

use crate::logging::debug_log;

/// Lightweight parser for .csproj files to extract TargetFramework, AssemblyName, and OutputType.
/// This is intentionally small and avoids heavy XML deps to stay WASM-friendly.

#[derive(Debug, Clone)]
pub enum OutputType {
    Exe,
    WinExe,
    Library,
}

#[derive(Debug, Clone)]
pub struct DotNetProject {
    pub target_framework: String,
    pub assembly_name: String,
    pub output_type: OutputType,
    pub project_path: PathBuf,
}

impl DotNetProject {
    pub fn from_csproj_text(text: &str, project_path: &Path) -> Self {
        let target_framework =
            extract_first_of_tags(text, &["TargetFramework", "TargetFrameworks"])
                .unwrap_or_else(|| {
                    // Fallback: Try to detect .NET SDK version or use latest LTS (net8.0)
                    // In practice, if TargetFramework is missing, the project is likely invalid,
                    // but we provide a reasonable default.
                    "net8.0".to_string()
                });

        let assembly_name = extract_first_of_tags(text, &["AssemblyName"]).unwrap_or_else(|| {
            project_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Program")
                .to_string()
        });

        let output_type = match extract_first_of_tags(text, &["OutputType"]).as_deref() {
            Some("Exe") => OutputType::Exe,
            Some("WinExe") => OutputType::WinExe,
            _ => OutputType::Library,
        };

        // If TargetFrameworks (plural) was present, it may be a semicolon list; pick the first one.
        let tf = if target_framework.contains(';') {
            target_framework
                .split(';')
                .next()
                .unwrap()
                .trim()
                .to_string()
        } else {
            target_framework
        };

        DotNetProject {
            target_framework: tf,
            assembly_name,
            output_type,
            project_path: project_path.to_path_buf(),
        }
    }

    /// Get the expected output path for a built assembly for the given configuration (Debug/Release).
    pub fn get_output_path(&self, configuration: &str) -> PathBuf {
        let ext = match self.output_type {
            OutputType::Exe | OutputType::WinExe => "exe",
            OutputType::Library => "dll",
        };

        let project_dir = self
            .project_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        project_dir
            .join("bin")
            .join(configuration)
            .join(&self.target_framework)
            .join(format!("{}.{}", &self.assembly_name, ext))
    }
}

fn extract_first_of_tags(text: &str, tags: &[&str]) -> Option<String> {
    for &tag in tags {
        if let Some(val) = extract_tag_value(text, tag) {
            return Some(val);
        }
    }
    None
}

fn extract_tag_value(text: &str, tag: &str) -> Option<String> {
    // Very small, tolerant XML tag extractor. It does not validate XML and will pick the
    // first matching <tag>...</tag> or <tag /> occurrence. It also handles windows-style
    // whitespace and attributes (it ignores attributes).

    // Search for opening tag like <Tag> or <Tag
    let open1 = format!("<{}>", tag);
    if let Some(start) = text.find(&open1) {
        if let Some(end) = text[start + open1.len()..].find(&format!("</{}>", tag)) {
            let val = &text[start + open1.len()..start + open1.len() + end];
            return Some(val.trim().to_string());
        }
    }

    // Try variant with attributes: e.g. <Tag Condition="...">value</Tag>
    let open_prefix = format!("<{} ", tag);
    if let Some(start) = text.find(&open_prefix) {
        if let Some(close_gt) = text[start..].find('>') {
            let inner_start = start + close_gt + 1;
            if let Some(end_tag) = text[inner_start..].find(&format!("</{}>", tag)) {
                let val = &text[inner_start..inner_start + end_tag];
                return Some(val.trim().to_string());
            }
        }
    }

    // Try self-closing (no value) -> ignore
    None
}

/// Check if the given path is a Unity project by looking for characteristic Unity directories and files.
/// A Unity project is identified by:
/// 1. Assets/ directory exists (checked by reading AssemblyDefinitions.json if available)
/// 2. ProjectSettings/ directory exists (checked by reading ProjectVersion.txt)
/// 3. Uses read_text_file since directory enumeration isn't available in WASM
pub fn is_unity_project(worktree: &zed::Worktree) -> bool {
    let root_path = worktree.root_path();
    
    // Check for Assets directory by trying to read a known file
    // Unity projects typically have Assets/csc.rsp or similar files
    let has_assets = worktree.read_text_file("Assets/csc.rsp").is_ok() ||
                    worktree.read_text_file("Assets/mcs.rsp").is_ok();
    
    // Check for ProjectSettings by trying to read ProjectVersion.txt
    let has_project_settings = worktree.read_text_file("ProjectSettings/ProjectVersion.txt").is_ok();
    
    // Unity project confirmed if we have ProjectSettings (strong indicator)
    if has_project_settings {
        debug_log!(worktree, "[csharp_roslyn] Unity project detected at: {root_path}");
        return true;
    }
    
    // Fallback: check for Assets directory indicators
    if has_assets {
        debug_log!(worktree, "[csharp_roslyn] Likely Unity project (Assets detected) at: {root_path}");
        return true;
    }
    
    false
}

/// Locate Unity-generated solution and project files, or suggest how to generate them.
/// Since directory enumeration isn't available in WASM, we check for common solution names.
/// Unity typically generates files in:
/// - Root directory: *.sln files (usually project name or "Assembly-CSharp.sln")
/// - Assembly-CSharp.csproj and other .csproj files
/// 
/// Returns either a path to existing .sln file or instructions for generation.
pub fn ensure_unity_project_files(worktree: &zed::Worktree) -> Result<String, String> {
    if !is_unity_project(worktree) {
        return Err("Not a Unity project".to_string());
    }
    
    let root_path = worktree.root_path();
    
    // Try common Unity solution file names
    let common_sln_names = [
        "Assembly-CSharp.sln",
        "Unity.sln", 
        // Also try the directory name as solution name
        &format!("{}.sln", Path::new(&root_path).file_name().unwrap_or_default().to_string_lossy()),
    ];
    
    for sln_name in &common_sln_names {
        if worktree.read_text_file(sln_name).is_ok() {
            debug_log!(worktree, "[csharp_roslyn] Found Unity solution: {sln_name}");
            return Ok(sln_name.to_string());
        }
    }
    
    // No .sln found - provide helpful instructions
    let instructions = format!(
        "Unity project detected at '{}' but no .sln files found.\n\
        \n\
        To generate project files:\n\
        1. Open the project in Unity Editor\n\
        2. Go to Edit → Preferences → External Script Editor\n\
        3. Set your editor and click 'Regenerate project files'\n\
        \n\
        Alternative: Run the helper script from your terminal:\n\
        scripts/generate-unity-projects.sh\n\
        \n\
        Once generated, the .sln file will be detected automatically.",
        root_path
    );
    
    debug_log!(worktree, "[csharp_roslyn] {instructions}");
    
    Err(instructions)
}

/// Generate Unity-specific OmniSharp configuration defaults
pub fn get_unity_omnisharp_config() -> serde_json::Value {
    serde_json::json!({
        "RoslynExtensionsOptions": {
            "enableDecompilationSupport": true,
            "enableImportCompletion": true,
            "enableAnalyzersSupport": true
        },
        "FormattingOptions": {
            "enableEditorConfigSupport": true
        },
        "FileOptions": {
            "excludeSearchPatterns": [
                "**/Library/**",
                "**/Temp/**", 
                "**/Logs/**",
                "**/obj/**",
                "**/bin/**"
            ]
        },
        "RoslynOptions": {
            "enableUnsafeCodeCompilation": false
        },
        "Plugins": {
            "Unity": {
                "enabled": true
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dotnet_project_from_csproj_basic() {
        let csproj_content = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <OutputType>Exe</OutputType>
    <AssemblyName>TestApp</AssemblyName>
  </PropertyGroup>
</Project>
"#;
        let project_path = std::path::Path::new("TestApp.csproj");
        let project = DotNetProject::from_csproj_text(csproj_content, project_path);
        
        assert_eq!(project.target_framework, "net8.0");
        assert_eq!(project.assembly_name, "TestApp");
        assert!(matches!(project.output_type, OutputType::Exe));
    }

    #[test]
    fn test_dotnet_project_from_csproj_library() {
        let csproj_content = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net6.0</TargetFramework>
    <OutputType>Library</OutputType>
    <AssemblyName>MyLibrary</AssemblyName>
  </PropertyGroup>
</Project>
"#;
        let project_path = std::path::Path::new("MyLibrary.csproj");
        let project = DotNetProject::from_csproj_text(csproj_content, project_path);
        
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.assembly_name, "MyLibrary");
        assert!(matches!(project.output_type, OutputType::Library));
    }

    #[test]
    fn test_dotnet_project_fallback_values() {
        let csproj_content = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
  </PropertyGroup>
</Project>
"#;
        let project_path = std::path::Path::new("TestProject.csproj");
        let project = DotNetProject::from_csproj_text(csproj_content, project_path);
        
        assert_eq!(project.target_framework, "net8.0"); // fallback
        assert_eq!(project.assembly_name, "TestProject"); // from filename
        assert!(matches!(project.output_type, OutputType::Library)); // fallback
    }

    #[test]
    fn test_dotnet_project_multiple_target_frameworks() {
        let csproj_content = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFrameworks>net6.0;net8.0;net462</TargetFrameworks>
    <AssemblyName>MultiTargetApp</AssemblyName>
  </PropertyGroup>
</Project>
"#;
        let project_path = std::path::Path::new("MultiTargetApp.csproj");
        let project = DotNetProject::from_csproj_text(csproj_content, project_path);
        
        assert_eq!(project.target_framework, "net6.0"); // first one
        assert_eq!(project.assembly_name, "MultiTargetApp");
    }

    #[test]
    fn test_unity_omnisharp_config_structure() {
        let config = get_unity_omnisharp_config();
        
        // Verify basic structure
        assert!(config.get("RoslynExtensionsOptions").is_some());
        assert!(config.get("FileOptions").is_some());
        assert!(config.get("Plugins").is_some());
        
        // Verify Unity-specific excludes
        let file_options = config.get("FileOptions").unwrap();
        let excludes = file_options.get("excludeSearchPatterns").unwrap().as_array().unwrap();
        
        assert!(excludes.iter().any(|v| v.as_str() == Some("**/Library/**")));
        assert!(excludes.iter().any(|v| v.as_str() == Some("**/Temp/**")));
        assert!(excludes.iter().any(|v| v.as_str() == Some("**/Logs/**")));
        
        // Verify Unity plugin is enabled
        let plugins = config.get("Plugins").unwrap();
        let unity = plugins.get("Unity").unwrap();
        assert_eq!(unity.get("enabled").unwrap().as_bool(), Some(true));
    }
}
