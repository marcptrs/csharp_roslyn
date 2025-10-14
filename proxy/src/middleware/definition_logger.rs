use crate::message::Message;
use crate::middleware::{Action, Middleware};
use anyhow::Result;

pub struct DefinitionLoggerMiddleware;

impl DefinitionLoggerMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for DefinitionLoggerMiddleware {
    fn name(&self) -> &str {
        "DefinitionLogger"
    }

    fn process_client_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }

    fn process_server_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }
}
