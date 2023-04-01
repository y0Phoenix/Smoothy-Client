use std::{process::{Child, ChildStdin, ChildStdout, Command, Stdio}, io::{BufWriter, BufReader}, sync::{Arc, Mutex}, thread::{JoinHandle, self}, time::Duration};

use sysinfo::{SystemExt, PidExt, Pid, ProcessExt, ProcessStatus};

pub struct Process {
    pub process: Child,
    pub stdin: BufWriter<ChildStdin>,
    pub stdout: Option<BufReader<ChildStdout>>,
    stop_checker_thread: JoinHandle<()>,
    internal_stopped: Arc<Mutex<bool>>,
}

impl Process {
    fn new() -> Self {
        let mut process = Command::new("node")
            .current_dir("server")
            .arg("main.js")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Internal Error Failed To Start Node App: Check That You Have node installed")
        ;
        let pid = process.id();

        let mut system = sysinfo::System::new();
        system.refresh_all();

        let stdin = BufWriter::new(process.stdin.take().expect("Internal IO Error: Failed To Aquire Nodejs Process Stdin"));
        let stdout = BufReader::new(process.stdout.take().expect("Internal IO Error: Failed To Aquire Nodejs Process Stdou"));

        let internal_stopped = Arc::new(Mutex::new(false));
        let internal_stopped_clone = Arc::clone(&internal_stopped);

        let stop_checker_thread = thread::Builder::new()
            .name("stopchecker".to_string())
            .spawn(move || {
                let internal_stopped = internal_stopped_clone;
                let process_info = system.process(Pid::from_u32(pid)).unwrap();
                loop {
                    let process_status = process_info.status();
                    if process_status != ProcessStatus::Idle && process_status != ProcessStatus::Run && process_status != ProcessStatus::Sleep {
                        let mut internal_stopped = internal_stopped.lock().unwrap();
                        *internal_stopped = true;
                        break;
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            })
            .unwrap()
        ;

        Self { 
            process, 
            stdin, 
            stdout: Some(stdout), 
            stop_checker_thread,
            internal_stopped
        }
    }   
    pub fn is_stopped(&self) -> bool {
        *self.internal_stopped.lock().unwrap()
    }
    pub fn kill(self) {
        self.stop_checker_thread.join().unwrap();
    }
}

pub struct ProcessStdout(pub BufReader<ChildStdout>);