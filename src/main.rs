use std::{io::BufReader, thread, time::Duration};

use chrono::{Local, NaiveTime};
use config::Config;
use log::{LogFile, log};
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

        let mut process_plugin = Process::new(config_data.server_folder.clone());

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
            log(log::LogType::INFO, "Restarting Smoothy");
            app.log_plugin.new_stdout(ProcessStdout(BufReader::new(app.process_plugin.stdout.take().unwrap())));
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn cmp_time(time: &String) -> bool {
    let curr_time = Local::now().format("%H:%M:%S").to_string();
    curr_time == *time 
}
