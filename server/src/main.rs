use std::{
  collections::{HashMap, HashSet},
  net::SocketAddr,
  sync::Arc,
};

use anyhow::Result;

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt, BufWriter},
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
      let body = serde_json::to_vec(&messages::server_to_client::ChatMessage {
        message_id: body.message_id,
        username: body.username,
        contents: body.contents,
      })?;

      futures::future::join_all(clients.iter_mut().map(|(socket_addr, write_half)| async {
        if *socket_addr != sender_addr {
          let mut writer = BufWriter::new(write_half);
          writer
            .write_u8(messages::MessageType::ChatMessage.as_u8())
            .await
            .unwrap();
          writer.write_u32(body.len() as u32).await.unwrap();
          writer.write_all(&body).await.unwrap();
          writer.flush().await.unwrap();
        }
      }))
      .await;
    }

    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  let chat_manager = ChatManager::new();

  let listener = TcpListener::bind("127.0.0.1:8080").await?;

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

  read_half.readable().await.unwrap();

  let message = io_utils::read_message(&mut read_half).await.unwrap();

  assert_eq!(message.r#type, messages::MessageType::JoinRoom);

  let body =
    serde_json::from_slice::<messages::client_to_server::JoinRoomMessage>(&message.body).unwrap();

  chat_manager.join_room(write_half, socket_addr, body).await;

  loop {
    read_half.readable().await.unwrap();

    let message = io_utils::read_message(&mut read_half).await.unwrap();

    match message.r#type {
      messages::MessageType::JoinRoom => {
        panic!("JoinRoom message received twice, this is a bug.");
      }
      messages::MessageType::ChatMessage => {
        let body =
          serde_json::from_slice::<messages::client_to_server::ChatMessage>(&message.body).unwrap();

        chat_manager
          .message_received(socket_addr, body)
          .await
          .unwrap();
      }
    }
  }
}
