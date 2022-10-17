use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
  pub message_id: String,
  pub username: String,
  pub contents: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReadMessage {
  pub message_id: String,
}
