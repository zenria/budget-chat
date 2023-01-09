use std::{
    collections::HashMap,
    fmt::Display,
    sync::{mpsc::Sender, Arc},
};

use parking_lot::Mutex;

#[derive(Default, Clone)]
pub struct Chatroom {
    inner: Arc<ChatroomImpl>,
}

impl Chatroom {
    /// Join the chatroom
    pub fn join(
        &self,
        nickname: String,
        message_sender: Sender<Message>,
    ) -> Result<Session, JoinError> {
        Ok(Session {
            id: self.inner.join(nickname, message_sender)?,
            chatroom_impl: self.inner.clone(),
        })
    }
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct SessionId(usize);

/// The current chat session.
///
/// Dropping the session will make the user leave the chatroom
pub struct Session {
    id: SessionId,
    chatroom_impl: Arc<ChatroomImpl>,
}

impl Session {
    pub fn send_message(&self, text: String) {
        self.chatroom_impl.send_message(self, text);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.chatroom_impl.leave(self.id);
    }
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

/// Chatroom private implementation
#[derive(Default)]
struct ChatroomImpl {
    connected_users: Mutex<HashMap<SessionId, (String, Sender<Message>)>>,
    session_count: Mutex<usize>,
}

impl ChatroomImpl {
    fn join(
        &self,
        nickname: String,
        message_sender: Sender<Message>,
    ) -> Result<SessionId, JoinError> {
        if nickname.len() == 0
            || nickname
                .chars()
                .any(|c| (c < 'a' || c > 'z') && (c < 'A' || c > 'Z') && (c < '0' || c > '9'))
        {
            return Err(JoinError::InvalidNickname);
        }
        let mut connected_users = self.connected_users.lock();

        for (n, _) in connected_users.values() {
            if n == &nickname {
                return Err(JoinError::DuplicateNickname);
            }
        }

        // send nicknames to the joining user
        let nicknames = connected_users
            .values()
            .map(|(n, _)| n)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let _ = message_sender.send(Message::ConnectedUsers(nicknames));

        // send all connected users the Joined message
        for (_, sender) in connected_users.values() {
            let _ = sender.send(Message::Joined(nickname.clone()));
        }

        let session_id = self.new_session_id();

        // register the joined user in our connected user database
        connected_users.insert(session_id, (nickname, message_sender));

        Ok(session_id)
    }

    fn new_session_id(&self) -> SessionId {
        let mut session_count = self.session_count.lock();
        *session_count += 1;
        SessionId(*session_count)
    }

    fn leave(&self, session: SessionId) {
        let mut connected_users = self.connected_users.lock();
        if let Some((nickname, _)) = connected_users.remove(&session) {
            // send all connected users the Joined message
            for (_, sender) in connected_users.values() {
                let _ = sender.send(Message::Left(nickname.clone()));
            }
        }
    }
    fn send_message(&self, from: &Session, text: String) {
        let connected_users = self.connected_users.lock();
        if let Some((from_nickname, _)) = connected_users.get(&from.id) {
            // send all connected users the Joined message
            for (to, (_, sender)) in connected_users.iter() {
                if to != &from.id {
                    let _ = sender.send(Message::Message {
                        from: from_nickname.clone(),
                        text: text.clone(),
                    });
                }
            }
        }
    }
}
