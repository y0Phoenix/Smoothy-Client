use std::{
    io::BufReader,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::Local;
use config::Config;
use input::{Input, InputCommand};
use logio::{
    err,
    file::{ArchiveType, Directory, FileName, LogioFile},
    info, Logger,
};
use process::Process;
use rusty_time::Timer;
// use smoothy::{get_servers, reset_global_data};
use sysinfo::{self, Pid, ProcessExt, System, SystemExt};

mod config;
mod input;
mod process;
mod smoothy;

pub trait Restart {
    fn restart(self) -> Self;
}

pub trait Kill {
    fn kill(self);
}

pub struct App {
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

        //let process_output = ProcessOutput {
        //    stderr: BufReader::new(process_plugin.stderr.take().unwrap()),
        //    stdout: BufReader::new(process_plugin.stdout.take().unwrap()),
        //};

        logio::init(
            Logger::new()
                .log_file(LogioFile::new(
                    FileName("log.log"),
                    Directory("logs"),
                    ArchiveType::Archive("archives"),
                ))
                .input_buf(Box::new(process_plugin.stderr.take().unwrap()))
                .input_buf(Box::new(process_plugin.stdout.take().unwrap()))
                .run(),
        );

        Self {
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
        logio::kill();
    }
    /// Returns true if the maximum crash count has been met in the specified time or false if it
    /// hasn't
    pub fn max_crash_count(&mut self, delta: Duration) -> bool {
        if self.process_plugin.is_stopped() {
            if self.crash_count == 0 {
                self.crash_count_timer.reset();
            }
            self.crash_count += 1;
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
        info!(
            "Smoothy's Crash Count Withtin The Timeframe Is Now {}",
            self.crash_count
        );
        // restart with a new stdout buff
        if new_buf {
            logio::new_input_bufs(vec![
                BufReader::new(Box::new(self.process_plugin.stderr.take().unwrap())),
                BufReader::new(Box::new(self.process_plugin.stderr.take().unwrap())),
            ]);
            // restart with the old buff replacednew buff
        }
        //else {
        //    self.log_plugin.new_process_out(ProcessOutput {
        //        stdout: BufReader::new(self.process_plugin.stdout.take().unwrap()),
        //        stderr: BufReader::new(self.process_plugin.stderr.take().unwrap()),
        //    })
        //}
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
                true => info!("Smoothy Successfully Killed"),
                false => err!("Failed To Kill Smoothy With PID: {}", pid,),
            },
            None => err!("No Valid Process Found For Smoothy's PID: {}", pid,),
        }
        default_panic(info);
    }));

    let mut instant = Instant::now();

    'main: loop {
        let input = app.input_plugin.input();
        let restart = cmp_time(&app.config_data.restart_time);
        let max_crash = app.max_crash_count(instant.elapsed());
        instant = Instant::now();
        let reset_crash_count = false;
        if max_crash {
            err!(
                "Smoothy Has Crashed The Max Amount `10` In Under 5 Minutes. Resetting Gloabal Data To Prevent Crashing"
                );
            // TODO reset DB instead of global file
            todo!("reset DB instead of global file");
            /*

            DEPRECATED code from nodejs smoothy

             */
            // // match reset_global_data(&app.config_data.global_data_file) {
            // //    Ok(_) => {
            // //        logio!(
            // //            log::LogType::Info,
            // //            "Successfully Reset Smoothy Global Data Attempting Restart",
            // //        );
            // //        reset_crash_count = true;
            // //    }
            // //    Err(_) => {
            // //        logio!(
            // //            log::LogType::Err,
            // //            "Something Went Wrong Resetting Smoothy Global Data. Closing Program To Prevent Further Crashing"
            // //        );
            // //        input = InputCommand::Exit;
            // //    }
            // //}
        }
        if app.process_plugin.is_stopped() {
            info!("Smoothy Stopped Unexpecteadly Attempting Restart",);
            app = app.restart(false, reset_crash_count);
        }
        if restart {
            if restart {
                info!(
                    "Restarting Smoothy From Configured Time Of {}",
                    app.config_data.restart_time
                );
            } else {
                info!("Restarting Smoothy");
            }
            app = app.restart(restart, reset_crash_count);
        }
        match input {
            InputCommand::Restart => {
                app = app.restart(false, reset_crash_count);
                info!("Restarting Smoothy");
            }
            // InputCommand::ListServers => {
            //     let servers = get_servers(app.config_data.global_data_file.to_string());
            //     match servers {
            //         Ok(servers) => {
            //             servers.print();
            //         }
            //         _ => logio!(log::LogType::Warn, "No Servers Found Or An Error Occured"),
            //     }
            // }
            InputCommand::Exit => {
                info!("Exiting App");
                info!("Killing Smoothy");
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
            _ => {}
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn cmp_time(time: &String) -> bool {
    let curr_time = Local::now().format("%H:%M:%S").to_string();
    curr_time == *time
}
