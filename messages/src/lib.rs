pub mod client_to_server;
pub mod server_to_client;

/// The type of the message.
#[derive(Debug, PartialEq, Eq)]
pub enum MessageType {
  JoinRoom,
  ChatMessage,
  MessageRead,
  MessageReceived,
}

impl MessageType {
  pub fn as_u8(&self) -> u8 {
    match self {
      MessageType::JoinRoom => 0,
      MessageType::ChatMessage => 1,
      MessageType::MessageRead => 2,
      MessageType::MessageReceived => 3,
    }
  }
}

impl From<u8> for MessageType {
  fn from(input: u8) -> Self {
    match input {
      0 => MessageType::JoinRoom,
      1 => MessageType::ChatMessage,
      2 => MessageType::MessageRead,
      3 => MessageType::MessageRead,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug)]
pub struct Message {
  pub r#type: MessageType,
  pub body: Vec<u8>,
}
