use crate::message::{Message, MessageId, ResponseMessage};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use dashmap::DashMap;
use serde_json::{json, Value};

pub struct DiagnosticsMiddleware {
    diagnostic_requests: DashMap<MessageId, ()>,
}

impl DiagnosticsMiddleware {
    pub fn new() -> Self {
        Self {
            diagnostic_requests: DashMap::new(),
        }
    }

    fn is_diagnostic_request(&self, method: &str) -> bool {
        method == "textDocument/diagnostic"
    }
}

impl Middleware for DiagnosticsMiddleware {
    fn name(&self) -> &str {
        "diagnostics"
    }

    fn process_client_message(&self, message: &Message) -> Result<Action> {
        if let Message::Request(req) = message {
            if self.is_diagnostic_request(&req.method) {
                self.diagnostic_requests.insert(req.id.clone(), ());
            }
        }
        Ok(Action::Continue)
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        if let Message::Response(resp) = message {
            if self.diagnostic_requests.remove(&resp.id).is_some() {
                if resp.result.is_none() || resp.result == Some(Value::Null) {
                    let mut new_resp = resp.clone();
                    new_resp.result = Some(json!({
                        "kind": "full",
                        "items": []
                    }));
                    return Ok(Action::Replace(Message::Response(new_resp)));
                }
            }
        }
        Ok(Action::Continue)
    }
}

impl Default for DiagnosticsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}