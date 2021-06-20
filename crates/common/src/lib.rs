#[cfg(feature = "binary")]
pub use bincode::deserialize as deserialize_binary;
#[cfg(feature = "binary")]
pub use bincode::serialize as serialize_binary;
#[cfg(feature = "json")]
pub use serde_json::from_str as deserialize_json;
#[cfg(feature = "json")]
pub use serde_json::to_string as serialize_json;

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

pub type RoomId = u32;
pub type UserId = u128;
pub type SessionId = u128;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RoomStateChange {
    UserJoined,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RoomMessage {
    UserInfo(UserId),
    RoomCreated(RoomId),
    NoSuchRoom(RoomId),
    RoomInfo(RoomInfo),
    UserJoined(UserId),
    UserLeft(UserId),
    UserVoted(UserId),
    CardsRevealed,
    Reset,
    Disconnected,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RoomRequest {
    CreateRoom,
    JoinRoom(RoomId),
    LeaveRoom,
    Vote(u32),
    RoomInfo,
    UserInfo,
    Reset,
    Reveal,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Card {
    display: Cow<'static, str>,
    value: Option<u32>,
}

impl Card {
    pub fn valueless<T: Into<Cow<'static, str>>>(display: T) -> Self {
        Self::new(display, None)
    }

    pub fn valued<T: Into<Cow<'static, str>>>(display: T, value: u32) -> Self {
        Self::new(display, Some(value))
    }

    pub fn new<T: Into<Cow<'static, str>>>(display: T, value: Option<u32>) -> Self {
        Self {
            display: display.into(),
            value,
        }
    }

    pub fn value(&self) -> Option<u32> {
        self.value
    }

    pub fn as_str(&self) -> &str {
        &self.display
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RoomInfo {
    pub id: RoomId,
    pub admin: UserId,
    pub users: Vec<RoomUser>,
    pub cards: Vec<Card>,

    pub revealed: bool,
}

impl RoomInfo {
    pub fn new(id: RoomId, admin: UserId) -> Self {
        Self {
            id,
            admin,
            users: vec![],
            cards: vec![],
            revealed: false,
        }
    }

    pub fn add_user(&mut self, user_id: UserId) -> bool {
        if !self.users.iter().any(|u| u.user_id == user_id) {
            self.users.push(RoomUser {
                user_id,
                vote: Vote::None,
            });
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct RoomUser {
    pub user_id: UserId,
    #[serde(default)]
    #[serde(skip_serializing_if = "vote_hidden")]
    pub vote: Vote,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Vote {
    None,
    Unknown,
    Hidden(u32),
    Revealed(u32),
}

impl Default for Vote {
    fn default() -> Self {
        Vote::Unknown
    }
}

impl Vote {
    pub fn value(&self) -> Option<u32> {
        match self {
            Self::Hidden(value) | Self::Revealed(value) => Some(*value),
            _ => None,
        }
    }
}

fn vote_hidden(vote: &Vote) -> bool {
    match vote {
        Vote::Hidden(_) => true,
        _ => false,
    }
}
