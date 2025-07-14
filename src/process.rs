#[allow(unused_imports)]
use std::{
    io::{BufReader, BufWriter},
    process::{ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use logio::{info, warn};

use crate::{Kill, Restart};

pub struct Process {
    // pub stdin: BufWriter<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
    pub pid: Arc<Mutex<u32>>,
    server_folder: String,
    kill_tx: Sender<bool>,
    stop_checker_thread: JoinHandle<()>,
    internal_stopped: Arc<Mutex<bool>>,
}

impl Process {
    pub fn new(server_folder: String) -> Self {
        let mut process = Command::new("cargo")
            .env("RUST_STDIO_BUFFERED", "0")
            .current_dir(server_folder.as_str())
            .args(["run", "--release"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Internal Error Failed To Start Rust App: Check That You Have rust installed");
        let pid = Arc::new(Mutex::new(process.id()));

        info!("Aquired PID: {}", process.id());

        // let stdin = BufWriter::new(
        //     process
        //         .stdin
        //         .take()
        //         .expect("Internal IO Error: Failed To Aquire Rust Process Stdin"),
        // );
        let stdout = process
            .stdout
            .take()
            .expect("Internal IO Error: Failed To Aquire Rust Process Stdou");
        let stderr = process
            .stderr
            .take()
            .expect("Internal IO Error: Failed To Aquire Rust Process Stderr");

        let internal_stopped = Arc::new(Mutex::new(false));
        let internal_stopped_clone = Arc::clone(&internal_stopped);

        let (kill_tx, kill_rx) = mpsc::channel::<bool>();

        let stop_checker_thread = thread::Builder::new()
            .name("stopchecker".to_string())
            .spawn(move || {
                let internal_stopped = internal_stopped_clone;
                loop {
                    if kill_rx.recv_timeout(Duration::from_secs(2)).is_ok() {
                        info!("Attemping To Kill Smoothy");
                        process
                            .kill()
                            .expect("Internale IO Error: Failed To Kill Smoothy Process");
                        process
                            .wait()
                            .expect("Internal IO Error: Failed To Kill Smoothy Process");
                        info!("Smoothy Successfully Killed");
                        break;
                    }
                    match process.try_wait() {
                        Ok(Some(_)) => {
                            warn!("Smoothy Stopped");
                            *internal_stopped.lock().unwrap() = true;
                            break;
                        }
                        Ok(None) => {}
                        Err(_) => {}
                    }
                }
            })
            .unwrap();

        Self {
            // stdin,
            stdout: Some(stdout),
            stderr: Some(stderr),
            stop_checker_thread,
            internal_stopped,
            kill_tx,
            server_folder,
            pid,
        }
    }
    pub fn is_stopped(&mut self) -> bool {
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
