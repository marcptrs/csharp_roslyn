pub mod capability_registration;
pub mod configuration;
pub mod custom;
pub mod definition_logger;
pub mod diagnostics;
pub mod document_lifecycle;
pub mod inlay_hints;
pub mod initialization;
pub mod project_restore;
pub mod refresh;
pub mod solution_loader;

use crate::message::Message;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Continue,
    Block,
    Replace(Message),
    Inject(Vec<Message>),
    RespondAndContinue(Message),
}

pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;

    fn process_client_message(&self, message: &Message) -> Result<Action> {
        let _ = message;
        Ok(Action::Continue)
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        let _ = message;
        Ok(Action::Continue)
    }
}

pub struct MiddlewarePipeline {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewarePipeline {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Box::new(middleware));
        self
    }

    pub fn process_client_message(&self, message: Message) -> Result<(Option<Message>, Vec<Message>)> {
        let mut current = message;
        let mut responses = Vec::new();

        for middleware in &self.middlewares {
            match middleware.process_client_message(&current)? {
                Action::Continue => {}
                Action::Block => return Ok((None, responses)),
                Action::Replace(new_msg) => current = new_msg,
                Action::Inject(messages) => {
                    responses.extend(messages);
                }
                Action::RespondAndContinue(response) => {
                    responses.push(response);
                }
            }
        }

        Ok((Some(current), responses))
    }

    pub fn process_server_message(&self, message: Message) -> Result<(Option<Message>, Vec<Message>)> {
        let mut current = message;
        let mut responses = Vec::new();

        for middleware in &self.middlewares {
            match middleware.process_server_message(&current)? {
                Action::Continue => {}
                Action::Block => return Ok((None, responses)),
                Action::Replace(new_msg) => current = new_msg,
                Action::Inject(messages) => {
                    responses.extend(messages);
                }
                Action::RespondAndContinue(response) => {
                    responses.push(response);
                }
            }
        }

        Ok((Some(current), responses))
    }
}

impl Default for MiddlewarePipeline {
    fn default() -> Self {
        Self::new()
    }
}
