use anyhow::Result;

use chrono::{DateTime, Utc};
use clap::Parser;
use console::Console;
use messages::Message;
use std::collections::VecDeque;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use uuid::Uuid;

mod console;

// TODO: duplicated in server/main.rs
const SERVER_ADDR: &str = "127.0.0.1:8080";

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Config {
  /// Your username.
  #[arg(long)]
  username: String,
  /// The room to which messages will be sent and received from.
  #[arg(long)]
  room: String,
}

struct ChatClient {
  config: Config,
  server_stream: TcpStream,
}

#[derive(Debug)]
pub struct MessageFromPeer {
  message_id: String,
  username: String,
  contents: String,
  received_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MessageFromClient {
  username: String,
  message_id: String,
  contents: String,
  sent_at: DateTime<Utc>,
}

impl ChatClient {
  async fn new(config: Config) -> Result<Self> {
    let mut client = Self {
      config,
      server_stream: TcpStream::connect(SERVER_ADDR).await?,
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

  async fn send(&mut self, message: MessageFromClient) -> Result<()> {
    let body = serde_json::to_vec(&messages::client_to_server::ChatMessage {
      message_id: message.message_id.clone(),
      username: self.config.username.clone(),
      // TODO: could avoid copying.
      contents: message.contents.clone(),
      room_id: self.config.room.clone(),
    })?;

    let mut writer = BufWriter::new(&mut self.server_stream);

    writer
      .write_u8(messages::MessageType::ChatMessage.as_u8())
      .await?;
    writer.write_u32(body.len() as u32).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;

    Ok(())
  }

  async fn recv(&mut self) -> Option<MessageFromPeer> {
    self.server_stream.readable().await.unwrap();

    let message = io_utils::read_message(&mut self.server_stream)
      .await
      .unwrap();

    match message.r#type {
      messages::MessageType::JoinRoom => unreachable!(),
      messages::MessageType::ChatMessage => {
        let body =
          serde_json::from_slice::<messages::server_to_client::ChatMessage>(&message.body).unwrap();

        Some(MessageFromPeer {
          message_id: body.message_id,
          username: body.username,
          contents: body.contents,
          received_at: Utc::now(),
        })
      }
    }
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  let config = Config::parse();
  let mut client = ChatClient::new(config.clone()).await?;

  let mut console = Console::new();
  let mut console_interval = tokio::time::interval(Duration::from_millis(200));

  loop {
    tokio::select! {
      message = client.recv() => {
        if let Some(message) = message {
          console.message_received(message);
        }
      }
      input = console.read_input() => {
        let message = MessageFromClient {
          username: config.username.clone(),
          message_id: Uuid::new_v4().to_string(),
          contents: input,
          sent_at: Utc::now()
        };
        client.send(message.clone()).await.unwrap();
        console.message_sent(message);
      }
      _ = console_interval.tick() => console.show_conversation()
    }
  }
}
