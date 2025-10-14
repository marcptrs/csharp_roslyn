use crate::message::Message;
use crate::middleware::{Action, Middleware};
use anyhow::Result;

pub struct InlayHintsMiddleware;

impl InlayHintsMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for InlayHintsMiddleware {
    fn name(&self) -> &str {
        "InlayHints"
    }

    fn process_client_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }

    fn process_server_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }
}

impl Default for InlayHintsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
