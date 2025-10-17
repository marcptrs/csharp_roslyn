use crate::message::{Message, NotificationMessage};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::info;

fn path_to_uri(path: &Path) -> String {
    if let Ok(url) = lsp_types::Url::from_file_path(path) {
        url.to_string()
    } else {
        format!("file://{}", path.display())
    }
}

pub struct SolutionLoaderMiddleware {
    workspace_root: Arc<Mutex<Option<PathBuf>>>,
    solution_path: Arc<Mutex<Option<PathBuf>>>,
}

impl SolutionLoaderMiddleware {
    pub fn new() -> Self {
        Self {
            workspace_root: Arc::new(Mutex::new(None)),
            solution_path: Arc::new(Mutex::new(None)),
        }
    }

    fn extract_project_files(&self, solution_path: &PathBuf) -> Vec<PathBuf> {
        let Ok(solution_content) = std::fs::read_to_string(solution_path) else {
            return Vec::new();
        };
        
        let solution_dir = solution_path.parent().unwrap_or(Path::new(""));
        let mut projects = Vec::new();
        
        for line in solution_content.lines() {
            if line.contains("Project(\"") {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 6 {
                    // Normalize backslashes to forward slashes (Windows .sln files use backslashes)
                    let project_rel_path = parts[5].replace('\\', "/");
                    if project_rel_path.ends_with(".csproj") {
                        let project_path = solution_dir.join(&project_rel_path);
                        projects.push(project_path);
                    }
                }
            }
        }
        
        projects
    }

    fn validate_solution(&self, solution_path: &PathBuf) -> Result<()> {
        let solution_content = std::fs::read_to_string(solution_path)
            .map_err(|e| anyhow::anyhow!("Failed to read solution file: {}", e))?;
        
        let project_count = solution_content.lines()
            .filter(|line| line.contains("Project(\""))
            .count();
        
        if project_count == 0 {
            return Err(anyhow::anyhow!("No projects found in solution"));
        }
        
        Ok(())
    }

    fn create_solution_and_project_notifications(&self, solution_path: PathBuf) -> Vec<Message> {
        let mut notifications = Vec::new();
        
        // Create solution/open notification
        let solution_uri = path_to_uri(&solution_path);
        
        notifications.push(Message::Notification(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "solution/open".to_string(),
            params: Some(json!({
                "solution": solution_uri
            })),
        }));
        
        // Extract and create project/open notification
        let project_files = self.extract_project_files(&solution_path);
        if !project_files.is_empty() {
            let project_uris: Vec<String> = project_files.iter()
                .map(|p| path_to_uri(p))
                .collect();
            
            notifications.push(Message::Notification(NotificationMessage {
                jsonrpc: "2.0".to_string(),
                method: "project/open".to_string(),
                params: Some(json!({
                    "projects": project_uris
                })),
            }));
        }
        
        notifications
    }
    
    fn extract_workspace_root(&self, message: &Message) -> Option<PathBuf> {
        if let Message::Request(req) = message {
            if req.method == "initialize" {
                if let Some(params) = &req.params {
                    // Try rootUri first
                    if let Some(root_uri) = params.get("rootUri").and_then(|v| v.as_str()) {
                        if let Ok(url) = lsp_types::Url::parse(root_uri) {
                            if let Ok(path) = url.to_file_path() {
                                return Some(path);
                            }
                        }
                    }
                    // Fall back to rootPath
                    if let Some(root_path) = params.get("rootPath").and_then(|v| v.as_str()) {
                        return Some(PathBuf::from(root_path));
                    }
                }
            }
        }
        None
    }
}

impl Middleware for SolutionLoaderMiddleware {
    fn name(&self) -> &str {
        "solution_loader"
    }

    fn process_client_message(&self, message: &Message) -> Result<Action> {
        if let Message::Request(req) = message {
            if req.method == "initialize" {
                if let Some(params) = &req.params {
                    if let Some(root) = self.extract_workspace_root(message) {
                        if let Ok(mut workspace) = self.workspace_root.lock() {
                            *workspace = Some(root);
                        }
                    }
                    
                    if let Some(init_options) = params.get("initializationOptions") {
                        if let Some(solution_uri) = init_options.get("solution").and_then(|v| v.as_str()) {
                            if let Ok(url) = lsp_types::Url::parse(solution_uri) {
                                if let Ok(solution_path) = url.to_file_path() {
                                    info!("Solution from initializationOptions: {}", solution_path.display());
                                    if let Ok(mut sol_path) = self.solution_path.lock() {
                                        *sol_path = Some(solution_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if let Message::Notification(notif) = message {
            if notif.method == "initialized" {
                let solution_path = self.solution_path.lock()
                    .ok()
                    .and_then(|guard| guard.clone());
                
                if let Some(solution_path) = solution_path {
                    info!("Using solution: {}", solution_path.display());
                    
                    if self.validate_solution(&solution_path).is_ok() {
                        let notifications = self.create_solution_and_project_notifications(solution_path);
                        if !notifications.is_empty() {
                            return Ok(Action::Inject(notifications));
                        }
                    }
                }
            }
        }
        
        Ok(Action::Continue)
    }

    fn process_server_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }
}
