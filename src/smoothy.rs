use std::fs::read_to_string;
use serde::*;

fn read_data(path: &String) -> GlobalData {
    let global_data = read_to_string(path).expect(format!("FS Error: Failed To Read Contents Of {:?}", path).as_str());

    serde_json::from_str::<GlobalData>(&global_data).expect("Error: Failed To Parsed Global Data")
}

pub fn get_servers(path: String) -> Result<Servers, ()> {
    let global_data = read_data(&path);

    let mut servers = Vec::new();

    for queue in global_data.queues {
        let mut curr_song: Option<Song> = None;
        for song in queue.currentsong {
            curr_song = Some(song);
        }
        
        servers.push(Server {
            name: queue.name.to_string(),
            songs: queue.songs,
            curr_song
        }); 
    }
    Ok(Servers(servers))
}  

pub struct Servers(Vec<Server>);

impl Servers {
    pub fn print(&self) {
        for server in self.0.iter() {
            server.print();
        }
    }
}

pub struct Server {
    name: String,
    songs: Vec<Song>,
    curr_song: Option<Song>
}

impl Server {
    pub fn print(&self) {
        println!("VC: {}", self.name);
        match self.curr_song.clone() {
            Some(curr_song) => {
                println!("Current Song: \n
                         \tTitle: {}\n
                         \tDuration: {}\n
                         \tUrl: {}", curr_song.title, curr_song.duration, curr_song.url)
                    ;
            },
            None => {
                println!("Current Song: No Song Currently Playing");
            }
        }
        println!("Songs In Queue");
        if self.songs.len() > 0 {
            for (i, song) in self.songs.iter().enumerate() {
                println!("{}\n
                         \tTitle: {}\n
                         \tDuration: {}\n
                         \tUrl: {}", i, song.title, song.duration, song.url)
                    ;
            }
        }
        else {
            println!("No Other Songs In Queue");
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalData {
    queues: Vec<WriteQueue>,
    disconnectIdles: Vec<WriteIdle>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WriteQueue {
    message: WriteMessage,
    id: String,
    name: String,
    songs: Vec<Song>,
    currentsong: [Song; 1],
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WriteIdle {
    message: WriteMessage,
    id: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WriteMessage {
    guild: Guild,
    author: Author,
    channelId: String,
    id: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Song {
    title: String,
    url: String,
    duration: u32
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Guild {
    id: String,
    name: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Author {
    id: String
}
