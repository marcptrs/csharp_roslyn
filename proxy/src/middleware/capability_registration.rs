use crate::message::{Message, ResponseMessage};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde_json::json;

pub struct CapabilityRegistrationMiddleware;

impl CapabilityRegistrationMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for CapabilityRegistrationMiddleware {
    fn name(&self) -> &str {
        "CapabilityRegistration"
    }

    fn process_client_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        if let Message::Request(req) = message {
            if req.method == "client/registerCapability" {
                let response = Message::Response(ResponseMessage {
                    jsonrpc: "2.0".to_string(),
                    id: req.id.clone(),
                    result: Some(json!(null)),
                    error: None,
                });
                
                return Ok(Action::Replace(response));
            }
        }
        
        Ok(Action::Continue)
    }
}
