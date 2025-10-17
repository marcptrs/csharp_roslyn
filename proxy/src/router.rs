use crate::connection::FramedConnection;
use crate::id_mapper::IdMapper;
use crate::message::{Message, MessageId, RequestMessage, ResponseMessage};
use crate::middleware::MiddlewarePipeline;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct Router {
    client_reader: Arc<Mutex<BufReader<tokio::io::Stdin>>>,
    client_writer: Arc<Mutex<tokio::io::Stdout>>,
    server_reader: Arc<Mutex<BufReader<ChildStdout>>>,
    server_writer: Arc<Mutex<ChildStdin>>,
    id_mapper: Arc<IdMapper>,
    pipeline: Arc<MiddlewarePipeline>,
}

impl Router {
    pub fn new(
        client_reader: tokio::io::Stdin,
        client_writer: tokio::io::Stdout,
        server_reader: ChildStdout,
        server_writer: ChildStdin,
        pipeline: MiddlewarePipeline,
    ) -> Self {
        Self {
            client_reader: Arc::new(Mutex::new(BufReader::new(client_reader))),
            client_writer: Arc::new(Mutex::new(client_writer)),
            server_reader: Arc::new(Mutex::new(BufReader::new(server_reader))),
            server_writer: Arc::new(Mutex::new(server_writer)),
            id_mapper: Arc::new(IdMapper::new()),
            pipeline: Arc::new(pipeline),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let client_to_server = self.route_client_to_server();
        let server_to_client = self.route_server_to_client();

        tokio::select! {
            result = client_to_server => result,
            result = server_to_client => result,
        }
    }

    async fn route_client_to_server(&self) -> Result<()> {
        loop {
            let message = {
                let mut reader = self.client_reader.lock().await;
                match read_lsp_message(&mut *reader).await? {
                    Some(msg) => msg,
                    None => {
                        info!("Client connection closed");
                        return Ok(());
                    }
                }
            };
            
            debug!("Client -> Server: {:?}", message.method());

            let (processed, responses) = match self.pipeline.process_client_message(message.clone()) {
                Ok((Some(msg), resps)) => (msg, resps),
                Ok((None, resps)) => {
                    debug!("Message blocked by middleware");
                    if !resps.is_empty() {
                        let mut writer = self.server_writer.lock().await;
                        for injected in resps {
                            write_lsp_message(&mut *writer, &injected).await
                                .context("Failed to write middleware-injected message to server")?;
                        }
                    }
                    continue;
                }
                Err(e) => {
                    error!("Middleware error: {}", e);
                    (message, vec![])
                }
            };

            let forwarded = self.map_client_message(processed)?;

            let mut writer = self.server_writer.lock().await;
            
            // Send injected messages BEFORE the original message
            // This ensures didOpen is sent before requests that need the document
            for injected in responses {
                write_lsp_message(&mut *writer, &injected).await
                    .context("Failed to write middleware-injected message to server")?;
            }
            
            write_lsp_message(&mut *writer, &forwarded).await
                .context("Failed to write to server")?;
        }
    }

    async fn route_server_to_client(&self) -> Result<()> {
        loop {
            let message = {
                let mut reader = self.server_reader.lock().await;
                match read_lsp_message(&mut *reader).await? {
                    Some(msg) => msg,
                    None => {
                        info!("Server connection closed");
                        return Ok(());
                    }
                }
            };

            info!("Server -> Client: {:?}", message.method());

            let (processed, responses) = match self.pipeline.process_server_message(message.clone()) {
                Ok((Some(msg), resps)) => (msg, resps),
                Ok((None, resps)) => {
                    debug!("Message blocked by middleware");
                    for response in resps {
                        let mut writer = self.server_writer.lock().await;
                        write_lsp_message(&mut *writer, &response).await
                            .context("Failed to write middleware response to server")?;
                    }
                    continue;
                }
                Err(e) => {
                    error!("Middleware error: {}", e);
                    (message.clone(), vec![])
                }
            };

            for response in responses {
                let mut writer = self.server_writer.lock().await;
                write_lsp_message(&mut *writer, &response).await
                    .context("Failed to write middleware response to server")?;
            }

            let is_server_request = matches!(message, Message::Request(_));
            let is_response = matches!(processed, Message::Response(_));
            
            if is_server_request && is_response {
                let mut writer = self.server_writer.lock().await;
                write_lsp_message(&mut *writer, &processed).await
                    .context("Failed to write response to server")?;
                continue;
            }

            let forwarded = match self.unmap_server_message(processed) {
                Ok(msg) => msg,
                Err(e) => {
                    warn!("Skipping response with unknown ID: {}", e);
                    continue;
                }
            };
            
            let mut writer = self.client_writer.lock().await;
            write_lsp_message(&mut *writer, &forwarded).await
                .context("Failed to write to client")?;
        }
    }

    fn map_client_message(&self, message: Message) -> Result<Message> {
        match message {
            Message::Request(mut req) => {
                let server_id = self.id_mapper.map_client_id(req.id.clone());
                req.id = server_id;
                Ok(Message::Request(req))
            }
            other => Ok(other),
        }
    }

    fn unmap_server_message(&self, message: Message) -> Result<Message> {
        match message {
            Message::Response(mut resp) => {
                let client_id = self
                    .id_mapper
                    .get_client_id(&resp.id)
                    .context(format!("Unknown server ID: {:?}", resp.id))?;

                self.id_mapper.remove(&resp.id);
                resp.id = client_id;
                Ok(Message::Response(resp))
            }
            other => Ok(other),
        }
    }
}

// Helper functions for reading/writing LSP messages without FramedConnection
async fn read_lsp_message<R>(reader: &mut BufReader<R>) -> Result<Option<Message>>
where
    R: AsyncReadExt + Unpin,
{
    use tokio::io::AsyncBufReadExt;
    
    let mut content_length: Option<usize> = None;
    let mut buffer = String::new();

    loop {
        buffer.clear();
        let bytes_read = reader
            .read_line(&mut buffer)
            .await
            .context("Failed to read header line")?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let line = buffer.trim();

        if line.is_empty() {
            break;
        }

        if let Some(length_str) = line.strip_prefix("Content-Length: ") {
            content_length = Some(
                length_str
                    .parse()
                    .context("Invalid Content-Length header")?,
            );
        }
    }

    let content_length = content_length.context("Missing Content-Length header")?;

    let mut content = vec![0u8; content_length];
    reader
        .read_exact(&mut content)
        .await
        .context("Failed to read message content")?;

    let message: Message =
        serde_json::from_slice(&content).context("Failed to deserialize message")?;

    Ok(Some(message))
}

async fn write_lsp_message<W>(writer: &mut W, message: &Message) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let content = serde_json::to_vec(message).context("Failed to serialize message")?;

    let header = format!("Content-Length: {}\r\n\r\n", content.len());

    writer
        .write_all(header.as_bytes())
        .await
        .context("Failed to write header")?;

    writer
        .write_all(&content)
        .await
        .context("Failed to write content")?;

    writer.flush().await.context("Failed to flush writer")?;

    Ok(())
}
