use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

use crate::MessageType;

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomMessage {
  pub room_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
  pub message_id: u64,
  pub username: String,
  pub room_id: String,
  pub contents: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReadMessage {
  pub message_id: u64,
  pub room_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReceivedMessage {
  pub message_id: u64,
  pub room_id: String,
}

pub async fn write_chat_message(
  writer: &mut (impl AsyncWrite + Unpin),
  message: ChatMessage,
) -> std::io::Result<()> {
  let mut writer = BufWriter::new(writer);

  writer.write_u8(MessageType::ChatMessage.as_u8()).await?;

  writer.write_u64(message.message_id).await?;

  writer.write_u32(message.room_id.len() as u32).await?;
  writer.write_all(message.room_id.as_bytes()).await?;

  writer.write_u32(message.username.len() as u32).await?;
  writer.write_all(message.username.as_bytes()).await?;

  writer.write_u32(message.contents.len() as u32).await?;
  writer.write_all(message.contents.as_bytes()).await?;

  writer.flush().await?;

  Ok(())
}

pub async fn write_message_received(
  writer: &mut (impl AsyncWrite + Unpin),
  message: MessageReceivedMessage,
) -> std::io::Result<()> {
  let mut writer = BufWriter::new(writer);

  writer
    .write_u8(MessageType::MessageReceived.as_u8())
    .await?;

  writer.write_u64(message.message_id).await?;

  writer.write_u32(message.room_id.len() as u32).await?;
  writer.write_all(message.room_id.as_bytes()).await?;

  writer.flush().await?;

  Ok(())
}

pub async fn write_message_read(
  writer: &mut (impl AsyncWrite + Unpin),
  message: MessageReadMessage,
) -> std::io::Result<()> {
  let mut writer = BufWriter::new(writer);

  writer.write_u8(MessageType::MessageRead.as_u8()).await?;

  writer.write_u64(message.message_id).await?;

  writer.write_u32(message.room_id.len() as u32).await?;
  writer.write_all(message.room_id.as_bytes()).await?;

  writer.flush().await?;

  Ok(())
}

pub async fn write_join_room_message(
  writer: &mut (impl AsyncWrite + Unpin),
  message: JoinRoomMessage,
) -> std::io::Result<()> {
  let mut writer = BufWriter::new(writer);

  writer.write_u8(MessageType::JoinRoom.as_u8()).await?;

  writer.write_u32(message.room_id.len() as u32).await?;
  writer.write_all(message.room_id.as_bytes()).await?;

  writer.flush().await?;

  Ok(())
}
