use crate::message::Message;
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde_json::Value;

const REFRESH_METHODS: &[&str] = &[
    "workspace/diagnostic/refresh",
    "workspace/codeLens/refresh",
    "workspace/inlayHint/refresh",
    "workspace/semanticTokens/refresh",
];

pub struct RefreshMiddleware;

impl RefreshMiddleware {
    pub fn new() -> Self {
        Self
    }

    fn is_refresh_method(&self, method: &str) -> bool {
        REFRESH_METHODS.contains(&method)
    }

    fn should_fix_params(&self, params: &Option<Value>) -> bool {
        match params {
            Some(Value::Array(_)) => true,
            Some(Value::Null) => true,
            _ => false,
        }
    }
}

impl Middleware for RefreshMiddleware {
    fn name(&self) -> &str {
        "refresh"
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        match message {
            Message::Request(req) => {
                if self.is_refresh_method(&req.method) && self.should_fix_params(&req.params) {
                    let mut new_req = req.clone();
                    new_req.params = None;
                    return Ok(Action::Replace(Message::Request(new_req)));
                }
            }
            Message::Notification(notif) => {
                if self.is_refresh_method(&notif.method) && self.should_fix_params(&notif.params) {
                    let mut new_notif = notif.clone();
                    new_notif.params = None;
                    return Ok(Action::Replace(Message::Notification(new_notif)));
                }
            }
            _ => {}
        }
        Ok(Action::Continue)
    }
}

impl Default for RefreshMiddleware {
    fn default() -> Self {
        Self::new()
    }
}