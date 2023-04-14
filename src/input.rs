use std::{thread::{JoinHandle, self}, sync::{Arc, Mutex, mpsc}};

use crate::Kill;

pub enum Command {
    Restart,
    ListServers,
    Exit,
    Invalid
} 

pub struct Input {
   check_input_thread: JoinHandle<()>,
   killed: Arc<Mutex<bool>>,
   input: Arc<Mutex<Option<Command>>>
}

impl Input {
    pub fn new() -> Self {
        let (input_rx, input_tx) = mpsc::channel::<String>();

        let input = Arc::new(Mutex::new(None));
        let killed = Arc::new(Mutex::new(false));

        let input_clone = Arc::clone(&input);
        let killed_clone = Arc::clone(&killed);

        let check_input_thread = thread::Builder::new()
            .name("checkinput".to_string())
            .spawn(move ||{
                let (global_input, killed) = (Arc::clone(&input_clone), Arc::clone(&killed_clone));
                loop {
                    let input = String::new();
                    if *killed.lock().unwrap() {
                        break;
                    }
                    std::io::stdin().read_line(&mut input).expect("IO Error: Failed To Read Input");
                    input = input.trim().to_string();
                    let global_input = *global_input.lock().unwrap(); 
                    if global_input.is_some() {
                        global_input = Some(Command::Invalid); 
                        continue;
                    }
                    match input.as_str() {
                       "restart" => global_input = Some(Command::Restart),
                       "list-servers" => global_input = Some(Command::ListServers),
                       "exit" | "stop" => global_input = Some(Command::Exit),
                       _ => global_input = Some(Command::Invalid)
                    }
                }
            });

        Self { 
            check_input_thread, 
            killed, 
            input 
        }
    }
    pub fn input(&mut self) -> Option<Command> {
        let mut input = *self.input.lock().unwrap();
        input.take() 
    }
}

impl Kill for Input {
    fn kill(self) {
        *self.killed.lock().unwrap() = true;
        self.check_input_thread.join().unwrap();
    }
}
