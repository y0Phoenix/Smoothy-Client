/*

THIS CODE IS DEPRECATED FROM THE OLD NODEJS SMOOTHY

*/



// use serde::*;
// use std::{
//     fs::{self, read_to_string, OpenOptions},
//     io::Write,
// };

// use crate::log::log;
// fn read_data(path: &String) -> GlobalData {
//     let global_data = read_to_string(path)
//         .unwrap_or_else(|_| panic!("FS Error: Failed To Read Contents Of {:?}", path));

//     serde_json::from_str::<GlobalData>(&global_data).expect("Error: Failed To Parsed Global Data")
// }

// pub fn reset_global_data(path: &String) -> Result<(), ()> {
//     if let Err(_) = fs::remove_file(path) {}
//     let mut file = match OpenOptions::new().create(true).write(true).open(path) {
//         Ok(file) => file,
//         Err(_) => {
//             log(
//                 crate::log::LogType::Err,
//                 "Error Opening New global.json file",
//             );
//             return Err(());
//         }
//     };
//     match file.write_all("{\"queues\": [], \"disconnectIdles\": []}".as_bytes()) {
//         Ok(_) => {}
//         Err(_) => return Err(()),
//     }
//     match file.flush() {
//         Ok(_) => return Ok(()),
//         Err(_) => return Err(()),
//     }
// }

// pub fn get_servers(path: String) -> Result<Servers, ()> {
//     let global_data = read_data(&path);

//     let mut servers = Vec::new();

//     for queue in global_data.queues {
//         let mut curr_song: Option<Song> = None;
//         for song in queue.currentsong {
//             curr_song = Some(song);
//         }

//         servers.push(Server {
//             name: queue.name.to_string(),
//             songs: queue.songs,
//             curr_song,
//         });
//     }
//     Ok(Servers(servers))
// }

// pub struct Servers(Vec<Server>);

// impl Servers {
//     pub fn print(&self) {
//         if self.0.is_empty() {
//             println!("Not Currently Connected To Any Server");
//         } else {
//             for (i, server) in self.0.iter().enumerate() {
//                 server.print(i);
//             }
//         }
//     }
// }

// pub struct Server {
//     name: String,
//     songs: Vec<Song>,
//     curr_song: Option<Song>,
// }

// impl Server {
//     pub fn print(&self, i: usize) {
//         println!("Server #: {}\nServer Name: {}", i + 1, self.name);
//         match self.curr_song.clone() {
//             Some(curr_song) => {
//                 println!(
//                     "Current Song:
//     Title: {}
//     Duration: {}
//     Url: {}",
//                     curr_song.title, curr_song.duration, curr_song.url
//                 );
//             }
//             None => {
//                 println!("Current Song: No Song Currently Playing");
//             }
//         }
//         println!("Songs In Queue");
//         if self.songs.len() > 1 {
//             for (i, song) in self.songs.iter().enumerate() {
//                 if i > 0 {
//                     println!(
//                         "{}
//     Title: {}
//     Duration: {}
//     Url: {}",
//                         i, song.title, song.duration, song.url
//                     );
//                 }
//             }
//         } else {
//             println!("No Other Songs In Queue");
//         }
//     }
// }

// #[allow(non_snake_case)]
// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct GlobalData {
//     queues: Vec<WriteQueue>,
//     disconnectIdles: Vec<WriteIdle>,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct WriteQueue {
//     message: WriteMessage,
//     id: String,
//     name: String,
//     songs: Vec<Song>,
//     currentsong: [Song; 1],
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct WriteIdle {
//     message: WriteMessage,
//     id: String,
// }

// #[allow(non_snake_case)]
// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct WriteMessage {
//     guild: Guild,
//     author: Author,
//     channelId: String,
//     id: String,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct Song {
//     title: String,
//     url: String,
//     duration: String,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct Guild {
//     id: String,
//     name: String,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct Author {
//     id: String,
// }
