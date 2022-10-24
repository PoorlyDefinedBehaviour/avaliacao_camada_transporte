use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tracing::error;

use anyhow::Result;

use tokio::{
  net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
  sync::Mutex,
};

struct ChatManager {
  // TODO: too much contention.
  rooms: Mutex<HashMap<String, HashMap<SocketAddr, OwnedWriteHalf>>>,
}

impl ChatManager {
  fn new() -> Arc<Self> {
    Arc::new(Self {
      rooms: Mutex::new(HashMap::new()),
    })
  }

  async fn join_room(
    &self,
    write_half: OwnedWriteHalf,
    socket_addr: SocketAddr,
    body: messages::client_to_server::JoinRoomMessage,
  ) {
    let mut rooms = self.rooms.lock().await;
    let entry = rooms.entry(body.room_id).or_insert_with(HashMap::default);
    entry.insert(socket_addr, write_half);
  }

  async fn message_received(
    &self,
    sender_addr: SocketAddr,
    body: messages::client_to_server::ChatMessage,
  ) -> Result<()> {
    let mut rooms = self.rooms.lock().await;

    if let Some(clients) = rooms.get_mut(&body.room_id) {
      let message = messages::server_to_client::ChatMessage {
        message_id: body.message_id,
        username: body.username,
        contents: body.contents,
      };

      let _results: Vec<std::io::Result<()>> =
        futures::future::join_all(clients.iter_mut().map(|(socket_addr, write_half)| async {
          if *socket_addr != sender_addr {
            println!(
              "server: writing message to socket_addr={:?} message={:?}",
              socket_addr.clone(),
              &message
            );
            messages::server_to_client::write_chat_message(write_half, &message).await?;
          }

          Ok(())
        }))
        .await;
    }

    Ok(())
  }

  async fn message_read(
    &self,
    sender_addr: SocketAddr,
    message: messages::client_to_server::MessageReadMessage,
  ) -> Result<()> {
    let mut rooms = self.rooms.lock().await;
    if let Some(clients) = rooms.get_mut(&message.room_id) {
      let message = messages::server_to_client::MessageReadMessage {
        message_id: message.message_id,
      };

      let _results: Vec<std::io::Result<()>> =
        futures::future::join_all(clients.iter_mut().map(|(socket_addr, write_half)| async {
          if *socket_addr != sender_addr {
            messages::server_to_client::write_message_read(write_half, &message).await?;
          }

          Ok(())
        }))
        .await;
    }

    Ok(())
  }

  async fn message_delivered(
    &self,
    sender_addr: SocketAddr,
    message: messages::client_to_server::MessageReceivedMessage,
  ) -> Result<()> {
    let mut rooms = self.rooms.lock().await;
    if let Some(clients) = rooms.get_mut(&message.room_id) {
      let message = messages::server_to_client::MessageDeliveredMessage {
        message_id: message.message_id,
      };

      let _results: Vec<std::io::Result<()>> =
        futures::future::join_all(clients.iter_mut().map(|(socket_addr, write_half)| async {
          if *socket_addr != sender_addr {
            messages::server_to_client::write_message_delivered(write_half, &message).await?;
          }

          Ok(())
        }))
        .await;
    }

    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let chat_manager = ChatManager::new();

  let listener = TcpListener::bind("0.0.0.0:8080").await?;

  loop {
    let (socket, socket_addr) = listener.accept().await?;
    tokio::spawn(handle_connection(
      socket,
      socket_addr,
      Arc::clone(&chat_manager),
    ));
  }
}

async fn handle_connection(
  socket: TcpStream,
  socket_addr: SocketAddr,
  chat_manager: Arc<ChatManager>,
) {
  let (mut read_half, write_half) = socket.into_split();

  if let Err(err) = read_half.readable().await {
    error!("socket is not readable. error={:?}", err);
  }

  let message = match messages::read_client_message(&mut read_half).await {
    Err(err) => {
      error!("unable to read first message from socket. error={:?}", err);
      return;
    }
    Ok(v) => v,
  };

  match message {
    messages::ClientToServerMessage::JoinRoom(message) => {
      chat_manager
        .join_room(write_half, socket_addr, message)
        .await;
    }
    message => panic!(
      "expected JoinRoom message, this is a bug. message={:?}",
      message
    ),
  };

  loop {
    if let Err(err) = read_half.readable().await {
      error!("socket is not readable. error={:?}", err);
    }

    let message = match messages::read_client_message(&mut read_half).await {
      Err(err) => {
        error!("unexpected message, this is a bug. error={:?}", err);
        return;
      }
      Ok(v) => v,
    };

    if let Err(err) = handle_message(&chat_manager, socket_addr, message).await {
      error!("unexpected error handling meessage. error={:?}", err);
    }
  }
}

async fn handle_message(
  chat_manager: &ChatManager,
  socket_addr: SocketAddr,
  message: messages::ClientToServerMessage,
) -> Result<()> {
  match message {
    messages::ClientToServerMessage::JoinRoom(_message) => {
      panic!("JoinRoom message received twice, this is a bug.");
    }
    messages::ClientToServerMessage::ChatMessage(message) => {
      chat_manager.message_received(socket_addr, message).await
    }
    messages::ClientToServerMessage::MessageReceived(message) => {
      chat_manager.message_delivered(socket_addr, message).await
    }
    messages::ClientToServerMessage::MessageRead(message) => {
      chat_manager.message_read(socket_addr, message).await
    }
  }
}
