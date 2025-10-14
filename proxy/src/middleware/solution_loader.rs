use crate::message::{Message, NotificationMessage};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

pub struct SolutionLoaderMiddleware {
    workspace_root: Arc<Mutex<Option<PathBuf>>>,
    solution_opened: Arc<Mutex<bool>>,
    pending_cs_files: Arc<Mutex<Vec<Message>>>,
}

impl SolutionLoaderMiddleware {
    pub fn new() -> Self {
        Self {
            workspace_root: Arc::new(Mutex::new(None)),
            solution_opened: Arc::new(Mutex::new(false)),
            pending_cs_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn find_solution_file(&self, workspace_root: &PathBuf) -> Option<PathBuf> {
        info!("Searching for solution files in: {}", workspace_root.display());
        
        let search_dirs = vec![
            workspace_root.clone(),
        ];
        
        for dir in search_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    
                    if path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "sln")
                        .unwrap_or(false)
                    {
                        info!("Found solution file at root: {}", path.display());
                        return Some(path);
                    }
                    
                    if path.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub_entry in sub_entries.filter_map(|e| e.ok()) {
                                let sub_path = sub_entry.path();
                                if sub_path.extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(|ext| ext == "sln")
                                    .unwrap_or(false)
                                {
                                    info!("Found solution file in subdirectory: {}", sub_path.display());
                                    return Some(sub_path);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        warn!("No solution file found in workspace or immediate subdirectories");
        None
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
        let solution_uri = format!("file://{}", solution_path.display());
        
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
                .map(|p| format!("file://{}", p.display()))
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
                        if root_uri.starts_with("file://") {
                            let path = root_uri.trim_start_matches("file://");
                            return Some(PathBuf::from(path));
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
        // Extract workspace root from 'initialize' request
        if let Some(root) = self.extract_workspace_root(message) {
            if let Ok(mut workspace) = self.workspace_root.lock() {
                *workspace = Some(root);
            }
        }
        
        // Detect 'initialized' notification from client → server
        if let Message::Notification(notif) = message {
            if notif.method == "initialized" {
                info!("Received 'initialized' notification, attempting solution discovery");
                
                // Get workspace root
                let workspace_root = self.workspace_root.lock()
                    .ok()
                    .and_then(|guard| guard.clone());
                
                if let Some(root) = workspace_root {
                    info!("Workspace root: {}", root.display());
                    
                    // Find solution file
                    if let Some(solution_path) = self.find_solution_file(&root) {
                        info!("Validating solution: {}", solution_path.display());
                        
                        // Validate solution
                        if self.validate_solution(&solution_path).is_ok() {
                            info!("Solution validated successfully, opening solution and projects");
                            
                            // Mark solution as opened
                            if let Ok(mut opened) = self.solution_opened.lock() {
                                *opened = true;
                            }
                            
                            // Create both solution/open and project/open notifications
                            let notifications = self.create_solution_and_project_notifications(solution_path);
                            if !notifications.is_empty() {
                                info!("Injecting {} notifications to Roslyn", notifications.len());
                                return Ok(Action::Inject(notifications));
                            }
                        } else {
                            warn!("Solution validation failed");
                        }
                    } else {
                        info!("No solution file found, Roslyn will auto-discover projects");
                    }
                } else {
                    warn!("No workspace root available");
                }
            }
            
            // Check if this is a didOpen for a .cs file
            if notif.method == "textDocument/didOpen" {
                if let Some(params) = &notif.params {
                    if let Some(text_doc) = params.get("textDocument") {
                        if let Some(uri) = text_doc.get("uri").and_then(|u| u.as_str()) {
                            if uri.ends_with(".cs") {
                                // Check if solution is opened
                                let opened = self.solution_opened.lock()
                                    .map(|guard| *guard)
                                    .unwrap_or(false);
                                
                                if !opened {
                                    if let Ok(mut pending) = self.pending_cs_files.lock() {
                                        pending.push(message.clone());
                                    }
                                    return Ok(Action::Block);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(Action::Continue)
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        if let Message::Notification(notif) = message {
            if notif.method == "workspace/projectInitializationComplete" {
                let has_pending = self.pending_cs_files.lock()
                    .map(|guard| !guard.is_empty())
                    .unwrap_or(false);
                
                if has_pending {
                    let pending = {
                        self.pending_cs_files.lock()
                            .map(|mut guard| {
                                let messages = guard.clone();
                                guard.clear();
                                messages
                            })
                            .unwrap_or_default()
                    };
                    
                    if !pending.is_empty() {
                        return Ok(Action::Inject(pending));
                    }
                }
            }
        }
        
        Ok(Action::Continue)
    }
}
