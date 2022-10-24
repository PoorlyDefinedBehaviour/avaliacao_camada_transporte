use anyhow::{anyhow, Result};

use chrono::{DateTime, Utc};
use clap::Parser;
use console::Console;

use tokio::net::{TcpSocket, TcpStream};
use tracing::{error, info};

mod console;

// TODO: duplicated in server/main.rs
const SERVER_ADDR: &str = "0.0.0.0:8080";

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

#[derive(Debug, Clone)]
pub struct MessageFromClient {
  username: String,
  message_id: u64,
  contents: String,
  sent_at: DateTime<Utc>,
}

fn try_bind_to_port(socket: &TcpSocket, port: u16) -> Result<()> {
  for i in 0..=255 {
    let addr = format!("127.0.0.{i}:{port}").parse()?;

    if socket.bind(addr).is_ok() {
      info!("socket bound to {:?}", &addr);
      return Ok(());
    }
  }

  Err(anyhow!("unable to bind socket to port. port={port}"))
}

impl ChatClient {
  async fn new(config: Config) -> Result<Self> {
    let server_stream = {
      let socket = TcpSocket::new_v4()?;

      if let Some(port) = config.port {
        try_bind_to_port(&socket, port)?;
      }

      socket.connect(SERVER_ADDR.parse()?).await?
    };

    let mut client = Self {
      config,
      server_stream,
      next_message_id: 0,
    };

    client.join_room().await?;

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

  async fn send_chat_message(
    &mut self,
    message: messages::client_to_server::ChatMessage,
  ) -> Result<()> {
    messages::client_to_server::write_chat_message(&mut self.server_stream, message).await?;

    Ok(())
  }

  async fn join_room(&mut self) -> Result<()> {
    let room_id = self.room().to_owned();

    messages::client_to_server::write_join_room_message(
      &mut self.server_stream,
      messages::client_to_server::JoinRoomMessage { room_id },
    )
    .await?;

    Ok(())
  }

  async fn mark_message_as_read(&mut self, message_id: u64, room_id: String) -> Result<()> {
    messages::client_to_server::write_message_read(
      &mut self.server_stream,
      messages::client_to_server::MessageReadMessage {
        message_id,
        room_id,
      },
    )
    .await?;

    Ok(())
  }

  async fn recv(&mut self) -> Result<Option<messages::ServerToClientMessage>> {
    self.server_stream.readable().await?;

    let message = messages::read_server_message(&mut self.server_stream).await?;

    if let messages::ServerToClientMessage::ChatMessage(ref message) = message {
      let room_id = self.room().to_string();

      messages::client_to_server::write_message_received(
        &mut self.server_stream,
        messages::client_to_server::MessageReceivedMessage {
          room_id,
          message_id: message.message_id,
        },
      )
      .await?;
    }

    Ok(Some(message))
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let mut client = ChatClient::new(Config::parse()).await?;

  let mut console = Console::new();

  loop {
    tokio::select! {
      message = client.recv() => {
        if let Ok(Some(message)) = message {
          match message {
            messages::ServerToClientMessage::ChatMessage(message) => {
              let message_id = message.message_id;
              console.message_received(message);

              if let Err(err) = client.mark_message_as_read(message_id,client.room().to_owned()).await {
                error!("unable to mark message as read. message_id={} error={:?}", message_id,err);
              }
            },
            messages::ServerToClientMessage::MessageDelivered(message) => {
              console.message_delivered(message.message_id);
            },
            messages::ServerToClientMessage::MessageRead(message) => {
              console.message_read(message.message_id);
            },
          }
        }
      }
      input = console.read_input() => {
        match input {
          Err(err) => {
            println!("unable to read input. error={:?}",err);
          }
          Ok(input) => {
            let message_id = client.next_message_id();

            let message = MessageFromClient {
              username: client.username()?,
              message_id,
              contents: input,
              sent_at: Utc::now()
            };

            client.send_chat_message( messages::client_to_server::ChatMessage {
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
  }
}
