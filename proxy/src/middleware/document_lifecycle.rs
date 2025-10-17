use super::{Action, Middleware};
use crate::message::{Message, NotificationMessage};
use anyhow::Result;
use lsp_types::{DidOpenTextDocumentParams, TextDocumentItem, DidCloseTextDocumentParams};
use std::collections::HashSet;
use std::sync::Mutex;

pub struct DocumentLifecycleMiddleware {
    opened_documents: Mutex<HashSet<String>>,
}

impl DocumentLifecycleMiddleware {
    pub fn new() -> Self {
        Self {
            opened_documents: Mutex::new(HashSet::new()),
        }
    }

    fn extract_uri_from_request(&self, message: &Message) -> Option<String> {
        if let Message::Request(req) = message {
            if let Some(params) = &req.params {
                if let Some(text_document) = params.get("textDocument") {
                    if let Some(uri) = text_document.get("uri") {
                        if let Some(uri_str) = uri.as_str() {
                            return Some(uri_str.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn create_did_open_notification(&self, uri: &str) -> Option<Message> {
        // Parse the URI and convert to file path properly (handles Windows paths)
        let parsed_uri = lsp_types::Url::parse(uri).ok()?;
        let uri_path = parsed_uri.to_file_path().ok()?;
        let content = std::fs::read_to_string(&uri_path).ok()?;
        
        let language_id = if uri.ends_with(".cs") {
            "csharp"
        } else {
            "plaintext"
        };

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: parsed_uri,
                language_id: language_id.to_string(),
                version: 0,
                text: content,
            },
        };

        let notif = NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "textDocument/didOpen".to_string(),
            params: Some(serde_json::to_value(params).ok()?),
        };

        Some(Message::Notification(notif))
    }
}

impl Middleware for DocumentLifecycleMiddleware {
    fn name(&self) -> &str {
        "DocumentLifecycle"
    }

    fn process_client_message(&self, message: &Message) -> Result<Action> {
        match message {
            Message::Notification(notif) => {
                match notif.method.as_str() {
                    "textDocument/didOpen" => {
                        if let Some(params) = &notif.params {
                            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params.clone()) {
                                let uri = params.text_document.uri.to_string();
                                if let Ok(mut docs) = self.opened_documents.lock() {
                                    docs.insert(uri);
                                }
                            }
                        }
                    }
                    "textDocument/didClose" => {
                        if let Some(params) = &notif.params {
                            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params.clone()) {
                                let uri = params.text_document.uri.to_string();
                                if let Ok(mut docs) = self.opened_documents.lock() {
                                    docs.remove(&uri);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Message::Request(_) => {
                if let Some(uri) = self.extract_uri_from_request(message) {
                    if let Ok(docs) = self.opened_documents.lock() {
                        if !docs.contains(&uri) && uri.ends_with(".cs") {
                            drop(docs);
                            
                            if let Some(did_open) = self.create_did_open_notification(&uri) {
                                if let Ok(mut docs) = self.opened_documents.lock() {
                                    docs.insert(uri);
                                }
                                return Ok(Action::Inject(vec![did_open]));
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(Action::Continue)
    }
}
