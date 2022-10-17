use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use tokio::io::AsyncReadExt;

use crate::{MessageFromClient, MessageFromPeer};

pub struct Console {
  stdin: tokio::io::Stdin,
  messages: MaxLengthVec<Message>,
}

#[derive(Debug)]
enum Message {
  FromPeer {
    message_id: String,
    username: String,
    contents: String,
    received_at: DateTime<Utc>,
  },
  FromClient {
    username: String,
    message_id: String,
    contents: String,
    sent_at: DateTime<Utc>,
    delivered: bool,
    read: bool,
  },
}

impl Console {
  pub fn new() -> Self {
    Self {
      stdin: tokio::io::stdin(),
      messages: MaxLengthVec::new(5),
    }
  }

  pub async fn read_input(&mut self) -> String {
    let mut buffer = [0; 4096];
    let _bytes_read = self.stdin.read(&mut buffer).await.unwrap();
    String::from_utf8_lossy(&buffer).to_string()
  }

  pub fn message_received(&mut self, message: MessageFromPeer) {
    self.messages.push(Message::FromPeer {
      message_id: message.message_id,
      username: message.username,
      contents: message.contents,
      received_at: message.received_at,
    })
  }

  pub fn message_sent(&mut self, message: MessageFromClient) {
    self.messages.push(Message::FromClient {
      username: message.username,
      message_id: message.message_id,
      contents: message.contents,
      sent_at: message.sent_at,
      delivered: false,
      read: false,
    })
  }

  pub fn show_conversation(&self) {
    clear_console();

    for message in self.messages.items.iter() {
      match message {
        Message::FromPeer {
          message_id,
          username,
          contents,
          received_at,
        } => {
          println!("{}: {}", username, contents);
        }
        Message::FromClient {
          message_id,
          username,
          contents,
          sent_at,
          delivered,
          read,
        } => {
          let check = if *read { "✓" } else { "" };
          println!("        {}{}: {}", check, username, contents);
        }
      }
    }
  }
}

fn clear_console() {
  println!("\x1B[2J");
}

struct MaxLengthVec<T> {
  max_len: usize,
  items: VecDeque<T>,
}

impl<T> MaxLengthVec<T> {
  fn new(max_len: usize) -> Self {
    Self {
      max_len,
      items: VecDeque::new(),
    }
  }

  fn push(&mut self, value: T) {
    self.items.push_back(value);
    if self.items.len() > self.max_len {
      let _ = self.items.pop_front();
    }
  }
}
