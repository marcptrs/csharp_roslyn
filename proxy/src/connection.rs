use crate::message::Message;
use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub struct FramedConnection<R, W> {
    reader: BufReader<R>,
    writer: W,
}

impl<R, W> FramedConnection<R, W>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>> {
        let mut content_length: Option<usize> = None;
        let mut buffer = String::new();

        loop {
            buffer.clear();
            let bytes_read = self
                .reader
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
        self.reader
            .read_exact(&mut content)
            .await
            .context("Failed to read message content")?;

        let message: Message =
            serde_json::from_slice(&content).context("Failed to deserialize message")?;

        Ok(Some(message))
    }

    pub async fn write_message(&mut self, message: &Message) -> Result<()> {
        let content = serde_json::to_vec(message).context("Failed to serialize message")?;

        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        self.writer
            .write_all(header.as_bytes())
            .await
            .context("Failed to write header")?;

        self.writer
            .write_all(&content)
            .await
            .context("Failed to write content")?;

        self.writer.flush().await.context("Failed to flush writer")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::NotificationMessage;

    #[tokio::test]
    async fn test_write_notification() {
        let mut output = Vec::new();
        let input = std::io::Cursor::new(Vec::new());
        let mut conn = FramedConnection::new(input, &mut output);

        let notif = Message::Notification(NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "test/notification".to_string(),
            params: None,
        });

        conn.write_message(&notif).await.unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.starts_with("Content-Length: "));
        assert!(output_str.contains("test/notification"));
    }
}
