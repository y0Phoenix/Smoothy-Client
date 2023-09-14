use std::{
    io::BufReader,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::Local;
use config::Config;
use input::{Input, InputCommand};
use log::{log, LogFile};
use process::{Process, ProcessOutput};
use rusty_time::Timer;
use smoothy::{get_servers, reset_global_data};
use sysinfo::{self, Pid, ProcessExt, System, SystemExt};

mod config;
mod input;
mod log;
mod process;
mod smoothy;

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
    input_plugin: Input,
    pid: Arc<Mutex<u32>>,
    crash_count: u8,
    crash_count_timer: Timer,
}

impl App {
    fn new() -> Self {
        let config_data = Config::read();

        let input_plugin = Input::new();

        let mut process_plugin = Process::new(config_data.server_folder.clone());

        let process_output = ProcessOutput {
            stderr: BufReader::new(process_plugin.stderr.take().unwrap()),
            stdout: BufReader::new(process_plugin.stdout.take().unwrap()),
        };

        Self {
            log_plugin: LogFile::new(process_output, config_data.max_file_size),
            pid: process_plugin.pid.clone(),
            process_plugin,
            config_data,
            input_plugin,
            crash_count_timer: Timer::from_millis(300000),
            crash_count: 0,
        }
    }
    fn kill(self) {
        self.process_plugin.kill();
        self.input_plugin.kill();
        self.log_plugin.kill();
    }
    /// Returns true if the maximum crash count has been met in the specified time or false if it
    /// hasn't
    pub fn max_crash_count(&mut self, delta: Duration) -> bool {
        if self.process_plugin.is_stopped() {
            if self.crash_count == 0 {
                self.crash_count_timer.reset();
            }
            self.crash_count += 1;
            self.log_plugin.report_crash();
        }
        self.crash_count_timer.update(delta);
        if !self.crash_count_timer.ready && self.crash_count >= self.config_data.max_crash_count {
            return true;
        } else if self.crash_count_timer.ready {
            self.crash_count = 0;
        }
        false
    }
    pub fn restart(mut self, new_buf: bool, reset_crash_count: bool) -> Self {
        self.process_plugin = self.process_plugin.restart();
        if reset_crash_count {
            self.crash_count = 0;
        }
        log(
            log::LogType::Info,
            format!(
                "Smoothy's Crash Count Withtin The Timeframe Is Now {}",
                self.crash_count
            )
            .as_str(),
        );
        // restart with a new stdout buff
        if new_buf {
            self.log_plugin = LogFile::new(
                ProcessOutput {
                    stdout: BufReader::new(self.process_plugin.stdout.take().unwrap()),
                    stderr: BufReader::new(self.process_plugin.stderr.take().unwrap()),
                },
                self.config_data.max_file_size,
            );
        // restart with the old buff appended onto the new buff
        } else {
            self.log_plugin.new_process_out(ProcessOutput {
                stdout: BufReader::new(self.process_plugin.stdout.take().unwrap()),
                stderr: BufReader::new(self.process_plugin.stderr.take().unwrap()),
            })
        }
        self
    }
}

fn main() {
    let mut app = App::new();

    let pid = Arc::clone(&app.pid);

    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut system = System::new();
        system.refresh_all();
        let pid = *pid.lock().unwrap();
        match system.process(Pid::from(pid as usize)) {
            Some(process) => match process.kill() {
                true => log(log::LogType::Info, "Smoothy Successfully Killed"),
                false => log(
                    log::LogType::Err,
                    format!("Failed To Kill Smoothy With PID: {}", pid).as_str(),
                ),
            },
            None => log(
                log::LogType::Err,
                format!("No Valid Process Found For Smoothy's PID: {}", pid).as_str(),
            ),
        }
        default_panic(info);
    }));

    let mut instant = Instant::now();

    'main: loop {
        let mut input = app.input_plugin.input();
        let restart = cmp_time(&app.config_data.restart_time);
        let max_crash = app.max_crash_count(instant.elapsed());
        instant = Instant::now();
        let mut reset_crash_count = false;
        if max_crash {
            log(
                log::LogType::Err,
                "Smoothy Has Crashed The Max Amount `10` In Under 5 Minutes. Resetting Gloabal Data To Prevent Crashing"
                );
            match reset_global_data(&app.config_data.global_data_file) {
                Ok(_) => {
                    log(
                        log::LogType::Info,
                        "Successfully Reset Smoothy Global Data Attempting Restart",
                    );
                    reset_crash_count = true;
                }
                Err(_) => {
                    log(
                        log::LogType::Err,
                        "Something Went Wrong Resetting Smoothy Global Data. Closing Program To Prevent Further Crashing"
                    );
                    input = InputCommand::Exit;
                }
            }
        }
        if app.process_plugin.is_stopped() {
            log(
                log::LogType::Info,
                "Smoothy Stopped Unexpecteadly Attempting Restart",
            );
            app = app.restart(false, reset_crash_count);
        }
        if restart {
            if restart {
                log(
                    log::LogType::Info,
                    format!(
                        "Restarting Smoothy From Configured Time Of {}",
                        app.config_data.restart_time
                    )
                    .as_str(),
                );
            } else {
                log(log::LogType::Info, "Restarting Smoothy");
            }
            app = app.restart(restart, reset_crash_count);
        }
        match input {
            InputCommand::Restart => {
                app = app.restart(false, reset_crash_count);
                log(log::LogType::Info, "Restarting Smoothy");
            }
            InputCommand::ListServers => {
                let servers = get_servers(app.config_data.global_data_file.to_string());
                match servers {
                    Ok(servers) => {
                        servers.print();
                    }
                    _ => log(log::LogType::Warn, "No Servers Found Or An Error Occured"),
                }
            }
            InputCommand::Exit => {
                log(log::LogType::Info, "Exiting App");
                log(log::LogType::Info, "Killing Smoothy");
                app.kill();
                break 'main;
            }
            InputCommand::Help => {
                println!(
                    "'restart': restarts the Smoothy Process
'list-servers': lists the current connected servers
'exit': exits the app and kills the Smoothy Process"
                );
            }
            InputCommand::Invalid => {
                println!("Invalid InputCommand type Help for a list of commands");
            }
            InputCommand::None => {}
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn cmp_time(time: &String) -> bool {
    let curr_time = Local::now().format("%H:%M:%S").to_string();
    curr_time == *time
}
