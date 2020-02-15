use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use serenity::model::id::{ChannelId, UserId};

#[derive(Serialize, Deserialize, Debug)]
pub struct Room {
    channel_id: ChannelId,
    name: String,
    creator: UserId,
    expire_date: SystemTime,
}

impl Room {
    pub fn new(channel_id: ChannelId, name: String, creator: UserId) -> Room {
        let current_time = SystemTime::now();

        let expire_date = current_time
            .checked_add(Duration::from_secs(60 * 60 * 10))
            .unwrap();

        Room {
            channel_id,
            name,
            creator,
            expire_date,
        }
    }

    pub fn deserialize(string: &str) -> Result<Room, serde_json::error::Error> {
        serde_json::from_str::<Room>(string)
    }

    pub fn serialize(&self) -> Result<String, serde_json::error::Error> {
        serde_json::to_string(self)
    }

    pub fn id(&self) -> ChannelId {
        self.channel_id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn creator(&self) -> UserId {
        self.creator
    }

    pub fn is_almost_expired(&self) -> bool {
        let diff = self
            .expire_date
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::from_secs(0));

        diff < Duration::from_secs(60 * 60 * 12)
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expire_date
    }

    pub fn prolong(&mut self) {
        let new_expire_date = SystemTime::now()
            .checked_add(Duration::from_secs(60 * 60 * 13))
            .unwrap();

        self.expire_date = new_expire_date;
    }
}
