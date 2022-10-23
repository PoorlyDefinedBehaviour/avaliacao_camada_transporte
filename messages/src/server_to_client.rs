use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
  pub message_id: u64,
  pub username: String,
  pub contents: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReadMessage {
  pub message_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageDeliveredMessage {
  pub message_id: u64,
}
