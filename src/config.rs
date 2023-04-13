use std::fs::read_to_string;

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub struct Config {
    pub restart_time: String,
    pub global_data_file: String
}

impl Config {
    pub fn read() -> Self {
        let config_data = read_to_string("config/client_config.json").expect("FS Error: Failed To Read Config File");
        serde_json::from_str(&config_data).expect("Error: Failed To Parse Global Data")
    }
}