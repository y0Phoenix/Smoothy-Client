use std::fs::read_to_string;

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub struct Config {
    pub restart_time: String,
    pub global_data_file: String,
    pub server_folder: String,
    pub max_file_size: usize
}

impl Config {
    pub fn read() -> Self {
        let config_data = read_to_string("config/client_config.json").expect("FS Error: Failed To Read Config File");
        let mut data: Config = serde_json::from_str(&config_data).expect("Error: Failed To Parse Config Data");
        data.restart_time = format!("{}:00", data.restart_time);
        data
    }
}
