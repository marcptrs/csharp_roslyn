use crate::message::Message;
use crate::middleware::{Action, Middleware};
use anyhow::Result;

pub struct InitializationMiddleware;

impl InitializationMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for InitializationMiddleware {
    fn name(&self) -> &str {
        "initialization"
    }

    fn process_client_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }

    fn process_server_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }
}

impl Default for InitializationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
