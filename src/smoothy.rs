use std::{sync::{Arc, Mutex}, thread::{JoinHandle, self}, path::Path, fs::{read_to_string, File}, fmt::Debug, io::BufReader, time::Duration};

use serde::*;

use crate::Kill;

pub struct Smoothy {
    global_data: Arc<Mutex<GlobalData>>,
    is_killed: Arc<Mutex<bool>>,
    reader_thread: JoinHandle<()>

}

impl Smoothy {
    pub fn new(path: String) -> Self {
        let global_data = Arc::new(Mutex::new(Smoothy::read_data(&path)));
        let is_killed = Arc::new(Mutex::new(false));

        let is_killed_clone = Arc::clone(&is_killed);
        let global_data_clone = Arc::clone(&global_data);

        let reader_thread = thread::Builder::new()
            .name("globalreader".to_string())
            .spawn(move || {
                let is_killed = is_killed_clone;
                let global_data = global_data_clone;
                loop {
                    let new_global_data = Smoothy::read_data(&path);
                    let mut global_data = global_data.lock().unwrap();
                    *global_data = new_global_data;
                    thread::sleep(Duration::from_secs(10));
                    if *is_killed.lock().unwrap() {
                        break;
                    }
                }
            })
            .unwrap()
        ;

        Self { 
            global_data,
            is_killed,
            reader_thread 
        }
    }

    fn read_data(path: &String) -> GlobalData {
        let global_data = read_to_string(path).expect(format!("FS Error: Failed To Read Contents Of {:?}", path).as_str());

        serde_json::from_str::<GlobalData>(&global_data).expect("Error: Failed To Parsed Global Data")
    }
}

impl Kill for Smoothy {
    fn kill(self) {
        self.reader_thread.join().unwrap();
    }
}


#[derive(Serialize, Deserialize)]
pub struct GlobalData {
    queues: Vec<WriteQueue>,
    disconnectIdles: Vec<WriteIdle>
}

#[derive(Serialize, Deserialize)]
pub struct WriteQueue {
    message: WriteMessage,
    id: String,
}

#[derive(Serialize, Deserialize)]
pub struct WriteIdle {
    message: WriteMessage,
    id: String
}

#[derive(Serialize, Deserialize)]
pub struct WriteMessage {
    guild: Guild,
    author: Author,
    channelId: String,
    id: String
}

#[derive(Serialize, Deserialize)]
pub struct Guild {
    id: String
}

#[derive(Serialize, Deserialize)]
pub struct Author {
    id: String
}
