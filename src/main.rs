use std::{io::BufReader, thread, time::Duration};

use chrono::{Local, NaiveTime};
use config::Config;
use input::{Input, Command};
use log::{LogFile, log};
use process::{Process, ProcessStdout};
use smoothy::{ get_servers};

mod log;
mod process;
mod smoothy;
mod config;
mod input;  

pub trait Restart {
    fn restart(self) -> Self;
}

pub trait Kill {
    fn kill(self);
}

pub struct App {
    log_plugin: LogFile,
    process_plugin: Process,
    config_data: Config,
    input_plugin: Input
}

impl App {
    fn new() -> Self {
        let config_data = Config::read();

        let input_plugin = Input::new();

        let mut process_plugin = Process::new(config_data.server_folder.clone());

        let process_stdout = ProcessStdout(BufReader::new(process_plugin.stdout.take().unwrap()));

        Self { 
            log_plugin: LogFile::new(process_stdout), 
            process_plugin ,
            config_data,
            input_plugin
        }
    }
    fn kill(self) {
        self.process_plugin.kill();
        self.input_plugin.kill();
        self.log_plugin.kill();
    }
}

fn main() {
    let mut app = App::new();

    'main: loop {
        let input = app.input_plugin.input();
        let restart = cmp_time(&app.config_data.restart_time) || input == Command::Restart; 
        if restart || app.process_plugin.is_stopped() {
            app.process_plugin = app.process_plugin.restart();
            log(log::LogType::INFO, "Restarting Smoothy");
            if restart {
                app.log_plugin = LogFile::new(ProcessStdout(BufReader::new(app.process_plugin.stdout.take().unwrap())));
            }
            else {
                app.log_plugin.new_stdout(ProcessStdout(BufReader::new(app.process_plugin.stdout.take().unwrap())));
            }
        }
        match input {
            Command::Restart => {
                app.process_plugin = app.process_plugin.restart();
                log(log::LogType::INFO, "Restarting Smoothy");
                app.log_plugin.new_stdout(ProcessStdout(BufReader::new(app.process_plugin.stdout.take().unwrap())));
            },
            Command::ListServers => {
                let servers = get_servers(app.config_data.global_data_file.to_string());
                match servers {
                    Ok(servers) => {
                        servers.print();
                    },
                    _ => {}
                }
            },
            Command::Exit => {
                log(log::LogType::INFO, "Exiting App");
                log(log::LogType::INFO, "Killing Smoothy");
                app.kill();
                break 'main;
            },
            Command::Help => {
                println!("'restart': restarts the Smoothy Process\n
                         'list-servers': lists the current connected servers\n
                         'exit': exits the app and kills the Smoothy Process")
                    ;
            },
            Command::Invalid => {
                println!("Invalid Command type Help for a list of commands");
            },
            Command::None => {}
        }
    thread::sleep(Duration::from_secs(1));
    }
}

fn cmp_time(time: &String) -> bool {
    let curr_time = Local::now().format("%H:%M:%S").to_string();
    curr_time == *time 
}
