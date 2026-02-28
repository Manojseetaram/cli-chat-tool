use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Msg {
    pub msg_id: String,
    pub nick:   String,
    pub text:   String,
    pub ts:     i64,
    pub edited: bool,
}

pub type Store   = Arc<Mutex<HashMap<String, Msg>>>;
pub type Order   = Arc<Mutex<Vec<String>>>;
pub type Pending = Arc<Mutex<Vec<String>>>;