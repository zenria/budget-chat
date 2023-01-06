use std::{
    collections::HashMap,
    fmt::Display,
    sync::{mpsc::Sender, Mutex},
};

#[derive(Default)]
pub struct Chatroom {
    connected_users: Mutex<HashMap<String, Sender<Message>>>,
}

pub enum Message {
    /// sent to all connected user when a new user just joined
    Joined(String),
    /// sent to all connected user when an users just left
    Left(String),
    /// sent to the joining user right before they joins
    ConnectedUsers(Vec<String>),
    Message {
        from: String,
        text: String,
    },
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Joined(nick) => write!(f, "* {nick} joined the room"),
            Message::Left(nick) => write!(f, "* {nick} left the room"),
            Message::ConnectedUsers(users) => {
                write!(f, "* Welcome, the room contains: {}", users.join(", "))
            }
            Message::Message { from, text } => write!(f, "[{from}] {text}"),
        }
    }
}

pub enum JoinError {
    DuplicateNickname,
    InvalidNickname,
}

impl Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::DuplicateNickname => f.write_str("Nickname already used."),
            JoinError::InvalidNickname => {
                f.write_str("Nickname can only alphanumerical characters.")
            }
        }
    }
}

impl Chatroom {
    pub fn join(&self, nickname: String, message_sender: Sender<Message>) -> Result<(), JoinError> {
        let mut connected_users = self.connected_users.lock().unwrap();
        if connected_users.contains_key(&nickname) {
            return Err(JoinError::DuplicateNickname);
        }
        if nickname.len() == 0
            || nickname
                .chars()
                .any(|c| (c < 'a' || c > 'z') && (c < 'A' || c > 'Z') && (c < '0' || c > '9'))
        {
            return Err(JoinError::InvalidNickname);
        }

        // send nicknames to the joining user
        let nicknames = connected_users
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let _ = message_sender.send(Message::ConnectedUsers(nicknames));

        // send all connected users the Joined message
        for sender in connected_users.values() {
            let _ = sender.send(Message::Joined(nickname.clone()));
        }

        // register the joined user in our connected user database
        connected_users.insert(nickname, message_sender);

        Ok(())
    }

    pub fn leave(&self, nickname: String) {
        let mut connected_users = self.connected_users.lock().unwrap();
        connected_users.remove(&nickname);
        // send all connected users the Joined message
        for sender in connected_users.values() {
            let _ = sender.send(Message::Left(nickname.clone()));
        }
    }
    pub fn send_message(&self, from: String, text: String) {
        let connected_users = self.connected_users.lock().unwrap();
        // send all connected users the Joined message
        for (nickname, sender) in connected_users.iter() {
            if nickname != &from {
                let _ = sender.send(Message::Message {
                    from: from.clone(),
                    text: text.clone(),
                });
            }
        }
    }
}
