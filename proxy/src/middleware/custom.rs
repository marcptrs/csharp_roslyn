use crate::message::{Message, NotificationMessage};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde_json::json;
use tracing::{debug, warn};

pub struct CustomNotificationsMiddleware;

impl CustomNotificationsMiddleware {
    pub fn new() -> Self {
        Self
    }

    fn is_roslyn_custom_notification(&self, method: &str) -> bool {
        method.starts_with("workspace/_roslyn_")
            || method.starts_with("roslyn/")
            || method == "workspace/projectInitializationComplete"
    }

    fn should_block_notification(&self, method: &str) -> bool {
        matches!(method, "workspace/_roslyn_projectNeedsRestore")
    }

    fn should_convert_notification(&self, method: &str) -> bool {
        method == "workspace/_roslyn_openDocument"
    }

    fn should_log_notification(&self, method: &str) -> bool {
        matches!(
            method,
            "roslyn/beginMetadataAsSource" | "roslyn/endMetadataAsSource"
        )
    }

    fn log_notification(&self, notif: &NotificationMessage) {
        if let Some(params) = &notif.params {
            if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                let action = if notif.method == "roslyn/beginMetadataAsSource" {
                    "started"
                } else {
                    "ended"
                };
                debug!("BCL navigation {}: {}", action, uri);
            }
        }
    }

    fn convert_open_document(&self, notif: &NotificationMessage) -> Option<NotificationMessage> {
        let params = notif.params.as_ref()?;
        let uri = params.get("uri")?.as_str()?;
        let text = params.get("text")?.as_str()?;

        Some(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "textDocument/didOpen".to_string(),
            params: Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "csharp",
                    "version": 1,
                    "text": text
                }
            })),
        })
    }
}

impl Middleware for CustomNotificationsMiddleware {
    fn name(&self) -> &str {
        "custom-notifications"
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        match message {
            Message::Request(req) => {
                if self.is_roslyn_custom_notification(&req.method) {
                    if self.should_block_notification(&req.method) {
                        debug!("Blocking Roslyn custom request: {}", req.method);
                        return Ok(Action::Block);
                    }

                    debug!(
                        "Passing through unknown Roslyn request: {}",
                        req.method
                    );
                }
                Ok(Action::Continue)
            }
            Message::Notification(notif) => {
                if self.is_roslyn_custom_notification(&notif.method) {
                    if self.should_block_notification(&notif.method) {
                        debug!("Blocking Roslyn custom notification: {}", notif.method);
                        return Ok(Action::Block);
                    }

                    if self.should_log_notification(&notif.method) {
                        self.log_notification(notif);
                        return Ok(Action::Continue);
                    }

                    if self.should_convert_notification(&notif.method) {
                        if let Some(converted) = self.convert_open_document(notif) {
                            debug!(
                                "Converting {} to {}",
                                notif.method, converted.method
                            );
                            return Ok(Action::Replace(Message::Notification(converted)));
                        } else {
                            warn!(
                                "Failed to convert {}, blocking instead",
                                notif.method
                            );
                            return Ok(Action::Block);
                        }
                    }

                    debug!(
                        "Passing through unknown Roslyn notification: {}",
                        notif.method
                    );
                }
                Ok(Action::Continue)
            }
            Message::Response(_) => Ok(Action::Continue),
        }
    }
}

impl Default for CustomNotificationsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_blocks_project_restore_notification() {
        let middleware = CustomNotificationsMiddleware::new();

        let notification = Message::Notification(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "workspace/_roslyn_projectNeedsRestore".to_string(),
            params: Some(json!({"projectFilePath": "/path/to/project.csproj"})),
        });

        let action = middleware.process_server_message(&notification).unwrap();

        assert_eq!(action, Action::Block);
    }

    #[test]
    fn test_logs_metadata_notifications() {
        let middleware = CustomNotificationsMiddleware::new();

        let methods = vec![
            "roslyn/beginMetadataAsSource",
            "roslyn/endMetadataAsSource",
        ];

        for method in methods {
            let notification = Message::Notification(NotificationMessage {
                jsonrpc: "2.0".to_string(),
                method: method.to_string(),
                params: Some(json!({"uri": "file:///tmp/System.String.cs"})),
            });

            let action = middleware.process_server_message(&notification).unwrap();

            assert_eq!(action, Action::Continue, "Failed to allow: {}", method);
        }
    }

    #[test]
    fn test_converts_open_document_notification() {
        let middleware = CustomNotificationsMiddleware::new();

        let notification = Message::Notification(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "workspace/_roslyn_openDocument".to_string(),
            params: Some(json!({
                "uri": "file:///tmp/System.String.cs",
                "text": "namespace System { }"
            })),
        });

        let action = middleware.process_server_message(&notification).unwrap();

        match action {
            Action::Replace(Message::Notification(notif)) => {
                assert_eq!(notif.method, "textDocument/didOpen");
                let params = notif.params.unwrap();
                assert_eq!(params["textDocument"]["uri"], "file:///tmp/System.String.cs");
                assert_eq!(params["textDocument"]["languageId"], "csharp");
                assert_eq!(params["textDocument"]["text"], "namespace System { }");
            }
            _ => panic!("Expected Replace action"),
        }
    }

    #[test]
    fn test_passes_through_standard_notifications() {
        let middleware = CustomNotificationsMiddleware::new();

        let notification = Message::Notification(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "textDocument/publishDiagnostics".to_string(),
            params: None,
        });

        let action = middleware.process_server_message(&notification).unwrap();

        assert_eq!(action, Action::Continue);
    }

    #[test]
    fn test_blocks_malformed_open_document() {
        let middleware = CustomNotificationsMiddleware::new();

        let notification = Message::Notification(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "workspace/_roslyn_openDocument".to_string(),
            params: Some(json!({"invalid": "params"})),
        });

        let action = middleware.process_server_message(&notification).unwrap();

        assert_eq!(action, Action::Block);
    }
}
