use redis::Commands;
use serenity::model::id::{ChannelId, MessageId};

use crate::bot::room::Room;

pub struct DatabaseManager {
    client: redis::Client,
    connection: redis::Connection,
}

impl DatabaseManager {
    pub fn new() -> DatabaseManager {
        let r_client = redis::Client::open("redis://127.0.0.1/").expect("Cannot connect to redis");
        let r_con = r_client
            .get_connection()
            .expect("Cannot make a connection to redis");

        DatabaseManager {
            client: r_client,
            connection: r_con,
        }
    }

    pub fn room_add(&mut self, room: Room) -> redis::RedisResult<()> {
        let id: u64 = room.id().as_u64().clone();
        let data: String = room.serialize().expect("Could not serialize room");

        let _: () = self.connection.set(format!("room:{}", id), data)?;

        Ok(())
    }

    pub fn room_delete(&mut self, id: ChannelId) -> redis::RedisResult<()> {
        let _: () = self.connection.del(format!("room:{}", id.as_u64()))?;

        Ok(())
    }

    pub fn room_prolong(&mut self, id: u64) -> redis::RedisResult<()> {
        let room: String = self
            .connection
            .get(format!("room:{}", id))
            .expect("Could not find room in redis");

        let mut room: Room = Room::deserialize(room.as_str()).expect("Could not deserialize room");
        room.prolong();

        self.room_add(room)
    }

    pub fn room_list(&mut self) -> Vec<Room> {
        let keys: Vec<String> = self
            .connection
            .keys("room:*")
            .expect("Could not load keys from redis");

        if keys.len() == 1 {
            let room: String = self
                .connection
                .get(keys)
                .expect("Could not get rooms from redis");

            return vec![Room::deserialize(room.as_str()).expect("Could not deserialize room")];
        }

        let values: Vec<String> = self
            .connection
            .get(keys)
            .expect("Could not get rooms from redis");

        values
            .iter()
            .map(|k| {
                Room::deserialize(k)
                    .expect(format!("Could not deserialize room with key `{}`", k).as_str())
            })
            .collect()
    }

    pub fn message_add(
        &mut self,
        message_id: MessageId,
        channel_id: ChannelId,
    ) -> redis::RedisResult<()> {
        let id = message_id.as_u64().clone();
        let data = channel_id.as_u64().clone();

        let _: () = self.connection.set(format!("message:{}", id), data)?;
        Ok(())
    }

    pub fn message_get(&mut self, message_id: MessageId) -> redis::RedisResult<u64> {
        let id = message_id.as_u64().clone();
        self.connection.get(format!("message:{}", id))
    }

    pub fn message_delete(&mut self, message_id: &u64) -> redis::RedisResult<()> {
        let _: () = self.connection.del(format!("message:{}", message_id))?;

        Ok(())
    }
}
