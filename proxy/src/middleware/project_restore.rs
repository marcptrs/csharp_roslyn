use crate::message::{Message, MessageId, RequestMessage, ResponseMessage};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, RwLock};

#[derive(Debug, Deserialize)]
struct ProjectNeedsRestoreParams {
    #[serde(rename = "projectFilePaths")]
    project_file_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ProjectNeedsRestoreResponse {
    needed_restore: bool,
}

pub struct ProjectRestoreMiddleware {
    request_id: AtomicI64,
    in_progress: Arc<AtomicBool>,
    pending_uuids: Arc<RwLock<HashSet<String>>>,
}

impl ProjectRestoreMiddleware {
    pub fn new() -> Self {
        Self {
            request_id: AtomicI64::new(90000),
            in_progress: Arc::new(AtomicBool::new(false)),
            pending_uuids: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    fn next_request_id(&self) -> MessageId {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        MessageId::Number(id)
    }

    fn find_project_file(&self, source_file: &str) -> Option<PathBuf> {
        let path = Path::new(source_file);
        
        if path.extension().and_then(|e| e.to_str()) == Some("csproj") {
            return Some(path.to_path_buf());
        }
        
        let mut current = path.parent()?;
        
        loop {
            if let Ok(entries) = std::fs::read_dir(current) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.extension().and_then(|e| e.to_str()) == Some("csproj") {
                        return Some(entry_path);
                    }
                }
            }
            
            current = current.parent()?;
        }
    }

    fn transform_project_paths(&self, params: &Option<Value>) -> Option<Value> {
        let params = params.as_ref()?;
        let project_paths = params.get("projectFilePaths")?.as_array()?;
        
        let mut transformed_paths = Vec::new();
        for path in project_paths {
            if let Some(path_str) = path.as_str() {
                if let Some(project_file) = self.find_project_file(path_str) {
                    transformed_paths.push(project_file.to_string_lossy().to_string());
                } else {
                    transformed_paths.push(path_str.to_string());
                }
            }
        }
        
        let mut new_params = params.clone();
        new_params["projectFilePaths"] = json!(transformed_paths);
        
        if let Some(uuid) = params.get("UUID") {
            new_params["UUID"] = uuid.clone();
        }
        
        Some(new_params)
    }
}

impl Middleware for ProjectRestoreMiddleware {
    fn name(&self) -> &str {
        "ProjectRestore"
    }

    fn process_client_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        match message {
            Message::Request(req) if req.method == "workspace/_roslyn_projectNeedsRestore" => {
                let uuid = if let Some(params) = &req.params {
                    params
                        .get("UUID")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

                if let Some(uuid_str) = &uuid {
                    let mut pending = self.pending_uuids.write().unwrap();
                    if pending.contains(uuid_str) {
                        let response = Message::Response(ResponseMessage {
                            jsonrpc: "2.0".to_string(),
                            id: req.id.clone(),
                            result: Some(json!({"needed_restore": false})),
                            error: None,
                        });
                        
                        return Ok(Action::Replace(response));
                    }
                    pending.insert(uuid_str.to_string());
                }

                if self.in_progress.load(Ordering::SeqCst) {
                    let response = Message::Response(ResponseMessage {
                        jsonrpc: "2.0".to_string(),
                        id: req.id.clone(),
                        result: Some(json!({"needed_restore": false})),
                        error: None,
                    });
                    
                    return Ok(Action::Replace(response));
                }

                self.in_progress.store(true, Ordering::SeqCst);

                let transformed_params = self.transform_project_paths(&req.params);

                let response = Message::Response(ResponseMessage {
                    jsonrpc: "2.0".to_string(),
                    id: req.id.clone(),
                    result: Some(json!({"needed_restore": true})),
                    error: None,
                });

                return Ok(Action::Replace(response));
            }
            Message::Notification(notif) if notif.method == "workspace/_roslyn_projectNeedsRestore" => {
                let uuid = if let Some(params) = &notif.params {
                    params
                        .get("UUID")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

                if let Some(uuid_str) = &uuid {
                    let mut pending = self.pending_uuids.write().unwrap();
                    if pending.contains(uuid_str) {
                        return Ok(Action::Block);
                    }
                    pending.insert(uuid_str.to_string());
                }

                if self.in_progress.load(Ordering::SeqCst) {
                    return Ok(Action::Block);
                }

                self.in_progress.store(true, Ordering::SeqCst);

                let transformed_params = self.transform_project_paths(&notif.params);

                let restore_request = Message::Request(RequestMessage {
                    jsonrpc: "2.0".to_string(),
                    id: self.next_request_id(),
                    method: "workspace/_roslyn_restore".to_string(),
                    params: transformed_params.or_else(|| notif.params.clone()),
                });

                return Ok(Action::Inject(vec![restore_request]));
            }
            Message::Notification(notif) if notif.method == "workspace/_roslyn_restoreComplete" => {
                let uuid = if let Some(params) = &notif.params {
                    params
                        .get("UUID")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

                self.in_progress.store(false, Ordering::SeqCst);

                if let Some(uuid_str) = &uuid {
                    let mut pending = self.pending_uuids.write().unwrap();
                    pending.remove(uuid_str);
                }
            }
            _ => {}
        }
        Ok(Action::Continue)
    }
}
