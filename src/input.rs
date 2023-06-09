use std::{thread::{JoinHandle, self}, sync::{Arc, Mutex}};

use crate::Kill;

#[derive(PartialEq, Eq, Default, Clone, Copy, Debug)]
pub enum InputCommand {
    Restart,
    ListServers,
    Exit,
    Help,
    Invalid,
    #[default]
    None
} 

pub struct Input {
   check_input_thread: JoinHandle<()>,
   killed: Arc<Mutex<bool>>,
   input: Arc<Mutex<String>>
}

impl Input {
    pub fn new() -> Self {
        let input = Arc::new(Mutex::new(String::new()));
        let killed = Arc::new(Mutex::new(false));

        let input_clone = Arc::clone(&input);
        let killed_clone = Arc::clone(&killed);

        let check_input_thread = thread::Builder::new()
            .name("checkinput".to_string())
            .spawn(move ||{
                let (global_input, killed) = (Arc::clone(&input_clone), Arc::clone(&killed_clone));
                loop {
                    let mut input = String::new();
                    if *killed.lock().unwrap() {
                        break;
                    }
                    std::io::stdin().read_line(&mut input).expect("IO Error: Failed To Read Input");
                    let input = input.trim();
                    let mut global_input = global_input.lock().unwrap();
                    *global_input = input.to_string();
                    if input == "exit" || input == "stop" {
                        break;
                    }
                }
            })
            .unwrap()    
        ;

        Self { 
            check_input_thread, 
            killed, 
            input 
        }
    }
    pub fn input(&mut self) -> InputCommand {
        let mut input = self.input.lock().unwrap();
        if input.is_empty() {
            return InputCommand::default();
        }
        let return_value = match input.as_str() {
            "restart" => InputCommand::Restart,
            "list-servers" => InputCommand::ListServers,
            "exit" | "stop" => InputCommand::Exit,
            "help" => InputCommand::Help,
            _ => InputCommand::Invalid
        };
        input.clear();
        return_value
    }
}

impl Kill for Input {
    fn kill(self) {
        *self.killed.lock().unwrap() = true;
        self.check_input_thread.join().unwrap();
    }
}
