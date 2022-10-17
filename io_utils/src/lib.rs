use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

pub async fn read_message(
  reader: impl AsyncRead + Unpin,
) -> Result<messages::Message, tokio::io::Error> {
  let mut reader = BufReader::new(reader);

  let message_type = reader.read_u8().await?;

  let body_len = reader.read_u32().await?;

  let mut buffer = vec![0_u8; body_len as usize];

  let _bytes_read = reader.read_exact(&mut buffer).await?;

  Ok(messages::Message {
    r#type: messages::MessageType::from(message_type),
    body: buffer,
  })
}
