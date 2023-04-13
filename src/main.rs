use std::io::BufReader;

use chrono::{Local, NaiveTime};
use config::Config;
use log::LogFile;
use process::{Process, ProcessStdout};
use smoothy::Smoothy;

mod log;
mod process;
mod smoothy;
mod config;

pub trait Restart {
    fn restart(self) -> Self;
}

pub trait Kill {
    fn kill(self);
}

pub struct App {
    smoothy_plugin: Smoothy,
    log_plugin: LogFile,
    process_plugin: Process,
    config_data: Config
}

impl App {
    fn new() -> Self {
        let config_data = Config::read();

        let mut process_plugin = Process::new();

        let process_stdout = ProcessStdout(BufReader::new(process_plugin.stdout.take().unwrap()));

        Self { 
            smoothy_plugin: Smoothy::new(config_data.global_data_file.clone()), 
            log_plugin: LogFile::new(process_stdout), 
            process_plugin ,
            config_data
        }
    }
    fn kill(self) {
        self.log_plugin.kill();
        self.smoothy_plugin.kill();
        self.process_plugin.kill();
    }
}

fn main() {
    let mut app = App::new();

    'main: loop {
        if cmp_time(&app.config_data.restart_time) || app.process_plugin.is_stopped() {
            app.process_plugin = app.process_plugin.restart();
            app.log_plugin.new_stdout(ProcessStdout(BufReader::new(app.process_plugin.stdout.take().unwrap())));
        }
    }
}

fn cmp_time(time: &String) -> bool {
    let curr_time = Local::now().time();
    let time_to_check = NaiveTime::parse_from_str(time, "%H:%M").expect("Error: Invalid Time Format In Config File. Format Should Be %H:%M");
    curr_time == time_to_check 
}
