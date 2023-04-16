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
        if self.0.len() == 0 {
            println!("Not Currently Connected To Any Server");
        }
        else {
            for (i, server) in self.0.iter().enumerate() {
                server.print(i);
            }
        }
    }
}

pub struct Server {
    name: String,
    songs: Vec<Song>,
    curr_song: Option<Song>
}

impl Server {
    pub fn print(&self, i: usize) {
        println!("Server #: {}\nServer Name: {}", i + 1, self.name);
        match self.curr_song.clone() {
            Some(curr_song) => {
                println!(
"Current Song:
    Title: {}
    Duration: {}
    Url: {}", curr_song.title, curr_song.duration, curr_song.url)
                    ;
            },
            None => {
                println!("Current Song: No Song Currently Playing");
            }
        }
        println!("Songs In Queue");
        if self.songs.len() > 1 {
            for (i, song) in self.songs.iter().enumerate() {
                if i > 0 {
                    println!(
"{}
    Title: {}
    Duration: {}
    Url: {}", i, song.title, song.duration, song.url)
                        ;
                }
            }
        }
        else {
            println!("No Other Songs In Queue");
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalData {
    queues: Vec<WriteQueue>,
    disconnectIdles: Vec<WriteIdle>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteQueue {
    message: WriteMessage,
    id: String,
    name: String,
    songs: Vec<Song>,
    currentsong: [Song; 1],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteIdle {
    message: WriteMessage,
    id: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteMessage {
    guild: Guild,
    author: Author,
    channelId: String,
    id: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Song {
    title: String,
    url: String,
    duration: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Guild {
    id: String,
    name: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Author {
    id: String
}
