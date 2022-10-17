use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomMessage {
  pub room_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
  pub message_id: String,
  pub username: String,
  pub room_id: String,
  pub contents: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReadMessage {
  pub message_id: String,
}
