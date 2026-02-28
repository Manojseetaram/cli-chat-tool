

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]

pub struct Message {
    pub id : String,
    pub room_id : String,
    pub sender : String,
    pub timestamp : i64 ,
    pub encrypted_payload : Vec<u8>
}