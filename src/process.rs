use std::{process::{Child, ChildStdin, ChildStdout, Command, Stdio, ExitStatus}, io::{BufWriter, BufReader}, sync::{Arc, Mutex}, thread::{JoinHandle, self}, time::Duration};

use sysinfo::{SystemExt, PidExt, Pid, ProcessExt, ProcessStatus};

use crate::{Kill, Restart};

pub struct Process {
    pub stdin: BufWriter<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    stop_checker_thread: JoinHandle<()>,
    internal_stopped: Arc<Mutex<bool>>,
}

impl Process {
    pub fn new() -> Self {
        let mut process = Command::new("node")
            .current_dir("server")
            .arg("build/main.js")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Internal Error Failed To Start Node App: Check That You Have node installed")
        ;
        let pid = process.id();

        println!("{}", pid);

        let mut system = sysinfo::System::new_all();
        system.refresh_all();

        let stdin = BufWriter::new(process.stdin.take().expect("Internal IO Error: Failed To Aquire Nodejs Process Stdin"));
        let stdout = process.stdout.take().expect("Internal IO Error: Failed To Aquire Nodejs Process Stdou");

        let internal_stopped = Arc::new(Mutex::new(false));
        let internal_stopped_clone = Arc::clone(&internal_stopped);

        let stop_checker_thread = thread::Builder::new()
            .name("stopchecker".to_string())
            .spawn(move || {
                let internal_stopped = internal_stopped_clone;
                let _ = process.wait();
                println!("Smoothy Stopped");
                *internal_stopped.lock().unwrap() = true;
            })
            .unwrap()
        ;

        Self { 
            stdin, 
            stdout: Some(stdout), 
            stop_checker_thread,
            internal_stopped
        }
    }   
    pub fn is_stopped(&self) -> bool {
        *self.internal_stopped.lock().unwrap()
    }
}

impl Kill for Process {
    fn kill(self) {
        self.stop_checker_thread.join().unwrap();
    }
}

impl Restart for Process {
    fn restart(self) -> Self {
        self.stop_checker_thread.join().unwrap();
        Process::new()
    }
}

pub struct ProcessStdout(pub BufReader<ChildStdout>);
