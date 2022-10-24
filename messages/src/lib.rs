use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

pub mod client_to_server;
pub mod server_to_client;

pub const MAX_MESSAGE_BYTES: usize = 4096;

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
pub enum ClientToServerMessage {
  JoinRoom(client_to_server::JoinRoomMessage),
  ChatMessage(client_to_server::ChatMessage),
  MessageReceived(client_to_server::MessageReceivedMessage),
  MessageRead(client_to_server::MessageReadMessage),
}

#[derive(Debug)]
pub enum ServerToClientMessage {
  ChatMessage(server_to_client::ChatMessage),
  MessageDelivered(server_to_client::MessageDeliveredMessage),
  MessageRead(server_to_client::MessageReadMessage),
}

pub async fn read_client_message(
  reader: impl AsyncRead + Unpin,
) -> Result<ClientToServerMessage, tokio::io::Error> {
  let mut reader = BufReader::new(reader);

  let message_type = reader.read_u8().await?;

  match MessageType::from(message_type) {
    MessageType::JoinRoom => {
      let room_id_len = reader.read_u32().await?;

      let mut room_id = vec![0_u8; room_id_len as usize];
      reader.read_exact(&mut room_id).await?;

      Ok(ClientToServerMessage::JoinRoom(
        client_to_server::JoinRoomMessage {
          room_id: String::from_utf8_lossy(&room_id).to_string(),
        },
      ))
    }
    MessageType::ChatMessage => {
      let message_id = reader.read_u64().await?;

      let room_id_len = reader.read_u32().await?;
      let mut room_id = vec![0_u8; room_id_len as usize];
      reader.read_exact(&mut room_id).await?;

      let username_len = reader.read_u32().await?;
      let mut username = vec![0_u8; username_len as usize];
      reader.read_exact(&mut username).await?;

      let contents_len = reader.read_u32().await?;
      let mut contents = vec![0_u8; contents_len as usize];
      reader.read_exact(&mut contents).await?;

      Ok(ClientToServerMessage::ChatMessage(
        client_to_server::ChatMessage {
          message_id,
          room_id: String::from_utf8_lossy(&room_id).to_string(),
          username: String::from_utf8_lossy(&username).to_string(),
          contents: String::from_utf8_lossy(&contents).to_string(),
        },
      ))
    }
    MessageType::MessageRead => {
      let message_id = reader.read_u64().await?;

      let room_id_len = reader.read_u32().await?;
      let mut room_id = vec![0_u8; room_id_len as usize];
      reader.read_exact(&mut room_id).await?;

      Ok(ClientToServerMessage::MessageRead(
        client_to_server::MessageReadMessage {
          message_id,
          room_id: String::from_utf8_lossy(&room_id).to_string(),
        },
      ))
    }
    MessageType::MessageReceived => {
      let message_id = reader.read_u64().await?;

      let room_id_len = reader.read_u32().await?;
      let mut room_id = vec![0_u8; room_id_len as usize];
      reader.read_exact(&mut room_id).await?;

      Ok(ClientToServerMessage::MessageReceived(
        client_to_server::MessageReceivedMessage {
          message_id,
          room_id: String::from_utf8_lossy(&room_id).to_string(),
        },
      ))
    }
  }
}

pub async fn read_server_message(
  reader: impl AsyncRead + Unpin,
) -> Result<ServerToClientMessage, tokio::io::Error> {
  let mut reader = BufReader::new(reader);

  let message_type = reader.read_u8().await?;

  match MessageType::from(message_type) {
    MessageType::JoinRoom => unreachable!(),
    MessageType::ChatMessage => {
      let message_id = reader.read_u64().await?;

      let username_len = reader.read_u32().await?;
      let mut username = vec![0_u8; username_len as usize];
      reader.read_exact(&mut username).await?;

      let contents_len = reader.read_u32().await?;
      let mut contents = vec![0_u8; contents_len as usize];
      reader.read_exact(&mut contents).await?;

      Ok(ServerToClientMessage::ChatMessage(
        server_to_client::ChatMessage {
          message_id,
          username: String::from_utf8_lossy(&username).to_string(),
          contents: String::from_utf8_lossy(&contents).to_string(),
        },
      ))
    }
    MessageType::MessageRead => {
      let message_id = reader.read_u64().await?;
      Ok(ServerToClientMessage::MessageRead(
        server_to_client::MessageReadMessage { message_id },
      ))
    }
    MessageType::MessageReceived => {
      let message_id = reader.read_u64().await?;

      Ok(ServerToClientMessage::MessageDelivered(
        server_to_client::MessageDeliveredMessage { message_id },
      ))
    }
  }
}
