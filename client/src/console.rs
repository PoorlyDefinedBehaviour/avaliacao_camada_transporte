use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use tokio::io::AsyncReadExt;

use crate::MessageFromClient;

pub struct Console {
  stdin: tokio::io::Stdin,
  messages: MaxLengthVec<Message>,
}

#[derive(Debug)]
enum Message {
  FromPeer {
    username: String,
    contents: String,
    received_at: DateTime<Utc>,
  },
  FromClient {
    username: String,
    message_id: u64,
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
      messages: MaxLengthVec::new(50),
    }
  }

  pub async fn read_input(&mut self) -> std::io::Result<String> {
    let mut buffer = [0_u8; messages::MAX_MESSAGE_BYTES];
    let bytes_read = self.stdin.read(&mut buffer).await?;
    Ok(String::from_utf8_lossy(&buffer[0..bytes_read]).to_string())
  }

  pub fn message_received(&mut self, message: messages::server_to_client::ChatMessage) {
    self.messages.push(Message::FromPeer {
      username: message.username,
      contents: message.contents,
      received_at: Utc::now(),
    });

    self.show_conversation();
  }

  pub fn message_sent(&mut self, message: MessageFromClient) {
    self.messages.push(Message::FromClient {
      username: message.username,
      message_id: message.message_id,
      contents: message.contents,
      sent_at: message.sent_at,
      delivered: false,
      read: false,
    });

    self.show_conversation();
  }

  pub fn message_read(&mut self, read_message_id: u64) {
    for message in self.messages.items.iter_mut() {
      if let Message::FromClient {
        message_id, read, ..
      } = message
      {
        if *message_id <= read_message_id {
          *read = true;
        }
      }
    }

    self.show_conversation();
  }

  pub fn message_delivered(&mut self, delivered_message_id: u64) {
    for message in self.messages.items.iter_mut() {
      if let Message::FromClient {
        message_id,
        delivered,
        ..
      } = message
      {
        if *message_id <= delivered_message_id {
          *delivered = true;
        }
      }
    }

    self.show_conversation();
  }

  pub fn show_conversation(&self) {
    clear_console();

    for message in self.messages.items.iter() {
      match message {
        Message::FromPeer {
          username,
          contents,
          received_at,
          ..
        } => {
          println!("    [{}] {username}: {contents}", format_date(*received_at));
        }
        Message::FromClient {
          username,
          contents,
          sent_at,
          delivered,
          read,
          ..
        } => {
          let check = if *read {
            "✓✓"
          } else if *delivered {
            "✓"
          } else {
            ""
          };
          println!("[{}] {check} {username}: {contents}", format_date(*sent_at));
        }
      }
    }
  }
}

fn format_date(date: DateTime<Utc>) -> String {
  (date - chrono::Duration::hours(3))
    .format("%H:%M:%S")
    .to_string()
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
