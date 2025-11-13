use std::path::{Path, PathBuf};

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
