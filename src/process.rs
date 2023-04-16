use std::{process::{Child, ChildStdin, ChildStdout, Command, Stdio, ExitStatus}, io::{BufWriter, BufReader}, sync::{Arc, Mutex, mpsc::{self, Sender}}, thread::{JoinHandle, self}, time::Duration};

use crate::{Kill, Restart, log::log};

pub struct Process {
    pub stdin: BufWriter<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    server_folder: String,
    kill_tx: Sender<bool>,
    stop_checker_thread: JoinHandle<()>,
    internal_stopped: Arc<Mutex<bool>>,
}

impl Process {
    pub fn new(server_folder: String) -> Self {
        let mut process = Command::new("node")
            .current_dir(server_folder.as_str())
            .arg("build/main.js")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Internal Error Failed To Start Node App: Check That You Have node installed")
        ;
        let pid = process.id();

        log(crate::log::LogType::INFO, format!("Aquired PID: {pid}").as_str());

        let stdin = BufWriter::new(process.stdin.take().expect("Internal IO Error: Failed To Aquire Nodejs Process Stdin"));
        let stdout = process.stdout.take().expect("Internal IO Error: Failed To Aquire Nodejs Process Stdou");

        let internal_stopped = Arc::new(Mutex::new(false));
        let internal_stopped_clone = Arc::clone(&internal_stopped);

        let (kill_tx, kill_rx) = mpsc::channel::<bool>();

        let stop_checker_thread = thread::Builder::new()
            .name("stopchecker".to_string())
            .spawn(move || {
                let internal_stopped = internal_stopped_clone;
                loop {
                    match kill_rx.recv_timeout(Duration::from_secs(2)) {
                        Ok(_) => {
                            log(crate::log::LogType::INFO, "Attemping To Kill Smoothy");
                            process.kill().expect("Internale IO Error: Failed To Kill Smoothy Process");
                            process.wait().expect("Internal IO Error: Failed To Kill Smoothy Process");
                            log(crate::log::LogType::INFO, "Smoothy Successfully Killed");
                            break;
                        },
                        _ => {},
                    }
                    match process.try_wait() {
                        Ok(Some(_)) => {
                            println!("Smoothy Stopped");
                            *internal_stopped.lock().unwrap() = true;
                            break;
                        },
                        Ok(None) => {},
                        Err(_) => {},
                    }    
                }
            })
            .unwrap()
        ;

        Self { 
            stdin, 
            stdout: Some(stdout), 
            stop_checker_thread,
            internal_stopped,
            kill_tx,
            server_folder
        }
    }   
    pub fn is_stopped(&self) -> bool {
        *self.internal_stopped.lock().unwrap()
    }
}

impl Default for Process {
    fn default() -> Self {
        Self::new("server".to_string())
    }
}

impl Kill for Process {
    fn kill(self) {
        let _ = self.kill_tx.send(true);
        self.stop_checker_thread.join().unwrap();
    }
}

impl Restart for Process {
    fn restart(self) -> Self {
        let _ = self.kill_tx.send(true);
        self.stop_checker_thread.join().unwrap();
        drop(self.kill_tx);
        Process::new(self.server_folder)
    }
}

pub struct ProcessStdout(pub BufReader<ChildStdout>);
