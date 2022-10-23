use anyhow::Result;

use chrono::{DateTime, Utc};
use clap::Parser;
use console::Console;

use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::{TcpSocket, TcpStream};

mod console;

// TODO: duplicated in server/main.rs
const SERVER_ADDR: &str = "18.228.22.102:8080";

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Config {
  /// Your username.
  #[arg(long)]
  username: String,
  /// The room to which messages will be sent and received from.
  #[arg(long)]
  room: String,
  #[arg(long)]
  /// The port that the client should use.
  port: Option<u16>,
}

struct ChatClient {
  config: Config,
  server_stream: TcpStream,
  next_message_id: u64,
}

#[derive(Debug)]
pub enum MessageFromPeer {
  ChatMessage(PeerChatMessage),
  Read(PeerReadMessage),
  Received(PeerMessageReceivedMessage),
}

#[derive(Debug)]
pub struct PeerChatMessage {
  message_id: u64,
  username: String,
  contents: String,
  received_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct PeerReadMessage {
  message_id: u64,
}

#[derive(Debug)]
pub struct PeerMessageReceivedMessage {
  message_id: u64,
}

#[derive(Debug, Clone)]
pub struct MessageFromClient {
  username: String,
  message_id: u64,
  contents: String,
  sent_at: DateTime<Utc>,
}

impl ChatClient {
  async fn new(config: Config) -> Result<Self> {
    let server_stream = {
      let socket = TcpSocket::new_v4()?;

      if let Some(port) = config.port {
        socket.set_reuseport(true)?;
        socket.bind(format!("localhost:{port}").parse()?)?;
      }

      socket.connect(SERVER_ADDR.parse()?).await?
    };

    let mut client = Self {
      config,
      server_stream,
      next_message_id: 0,
    };

    let body = serde_json::to_vec(&messages::client_to_server::JoinRoomMessage {
      room_id: client.config.room.clone(),
    })?;

    let mut writer = BufWriter::new(&mut client.server_stream);
    writer
      .write_u8(messages::MessageType::JoinRoom.as_u8())
      .await?;
    writer.write_u32(body.len() as u32).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;

    Ok(client)
  }

  fn next_message_id(&mut self) -> u64 {
    let id = self.next_message_id;
    self.next_message_id += 1;
    id
  }

  fn port(&self) -> std::io::Result<u16> {
    self.server_stream.local_addr().map(|addr| addr.port())
  }

  fn room(&self) -> &str {
    &self.config.room
  }

  fn username(&self) -> std::io::Result<String> {
    Ok(format!("{}({})", &self.config.username, self.port()?))
  }

  async fn send<M>(&mut self, message_type: messages::MessageType, message: M) -> Result<()>
  where
    M: serde::Serialize,
  {
    let body = serde_json::to_vec(&message)?;

    let mut writer = BufWriter::new(&mut self.server_stream);

    writer.write_u8(message_type.as_u8()).await?;
    writer.write_u32(body.len() as u32).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;

    Ok(())
  }

  async fn recv(&mut self) -> Result<Option<MessageFromPeer>> {
    self.server_stream.readable().await.unwrap();

    let message = io_utils::read_message(&mut self.server_stream)
      .await
      .unwrap();

    match message.r#type {
      messages::MessageType::JoinRoom => unreachable!(),
      messages::MessageType::MessageReceived => {
        let body =
          serde_json::from_slice::<messages::server_to_client::MessageReadMessage>(&message.body)?;

        Ok(Some(MessageFromPeer::Received(
          PeerMessageReceivedMessage {
            message_id: body.message_id,
          },
        )))
      }
      messages::MessageType::MessageRead => {
        let body =
          serde_json::from_slice::<messages::server_to_client::MessageReadMessage>(&message.body)?;

        Ok(Some(MessageFromPeer::Read(PeerReadMessage {
          message_id: body.message_id,
        })))
      }
      messages::MessageType::ChatMessage => {
        let body =
          serde_json::from_slice::<messages::server_to_client::ChatMessage>(&message.body)?;

        self
          .send(
            messages::MessageType::MessageReceived,
            messages::client_to_server::MessageReceivedMessage {
              room_id: self.room().to_owned(),
              message_id: body.message_id,
            },
          )
          .await?;

        Ok(Some(MessageFromPeer::ChatMessage(PeerChatMessage {
          message_id: body.message_id,
          username: body.username,
          contents: body.contents,
          received_at: Utc::now(),
        })))
      }
    }
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  let mut client = ChatClient::new(Config::parse()).await?;

  let mut console = Console::new();

  loop {
    tokio::select! {
      message = client.recv() => {
        if let Ok(Some(message)) = message {
          match message {
            MessageFromPeer::Received(PeerMessageReceivedMessage { message_id }) => {
              console.message_delivered(message_id);
            }
            MessageFromPeer::Read(PeerReadMessage { message_id }) => {
              console.message_read(message_id);
            },
            MessageFromPeer::ChatMessage(message) => {
              let message_id = message.message_id;
              console.message_received(message);

              client.send(messages::MessageType::MessageRead, messages::client_to_server::MessageReadMessage {
                message_id,
                room_id: client.room().to_owned()
              })
              .await
              .expect("error marking message as read");
            }
          }

        }
      }
      input = console.read_input() => {
        let message_id = client.next_message_id();

        let message = MessageFromClient {
          username: client.username()?,
          message_id,
          contents: input,
          sent_at: Utc::now()
        };

        client.send(messages::MessageType::ChatMessage, messages::client_to_server::ChatMessage {
          message_id,
          username: message.username.clone(),
          contents: message.contents.clone(),
          room_id: client.room().to_owned()
        })
        .await?;

        console.message_sent(message);

      }
    }
  }
}
