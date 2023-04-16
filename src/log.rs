use std::{io::{BufWriter, Write, BufReader, BufRead}, fs::{File, OpenOptions, read_dir, self, create_dir}, thread::{JoinHandle, self}, sync::{mpsc::{self, Sender}, Arc, Mutex},  time::Duration};

use chrono::{Local, Timelike, Datelike};

use crate::{process::ProcessStdout, Kill};

pub struct LogFile {
    tx_thread: JoinHandle<()>,
    rx_thread: JoinHandle<()>,
    flusher_thread: JoinHandle<()>,
    new_stdout_tx: Sender<ProcessStdout>,
    killed: Arc<Mutex<bool>>
}

impl LogFile {
    pub fn new(stdout: ProcessStdout) -> Self {
        let mut new_log = true;
        if fs::metadata("logs").is_err() {
            create_dir("logs").expect("FS Error: Failed To Create logs Directory");
        }
        let out_file = match OpenOptions::new()
            .write(true)
            .open("logs/log.txt") 
        {
            Ok(file) => {
                new_log = false;
                file
            },
            Err(_) => File::create("logs/log.txt").expect("Internal Error: Error Creating New Log File")   
        };

        if !new_log {
            LogFile::archive_log(out_file.try_clone().unwrap());
        }
        let killed = Arc::new(Mutex::new(false));
        let (killed_clone_1, killed_clone_2) = (Arc::clone(&killed), Arc::clone(&killed));
        
        let out_buf = Arc::new(Mutex::new(BufWriter::new(out_file.try_clone().unwrap())));
        let out_buf_clone = Arc::clone(&out_buf);
        let out_buf_lines = Arc::new(Mutex::new(0));
        let out_buf_lines_clone = Arc::clone(&out_buf_lines);

        let (stdout_tx, stdout_rx) = mpsc::channel::<String>();
        let (new_stdout_tx, new_stdout_rx) = mpsc::channel::<ProcessStdout>();

        let tx_thread = thread::Builder::new()
            .name("stdoutreader".to_string())
            .spawn(move || {
                let mut stdout_buf = stdout.0;
                loop {
                    if let Ok(new_stdout) = new_stdout_rx.recv_timeout(Duration::from_secs(1)) {
                        stdout_buf = new_stdout.0;
                    }
                    if stdout_buf.buffer().len() >= stdout_buf.capacity() {
                        let stdout = stdout_buf.into_inner();
                        stdout_buf = BufReader::new(stdout);
                    }
                    let mut output = String::new();
                    stdout_buf.read_line(&mut output).expect("Internal IO Error: Error Reading Line From Child Process stdout");
                    if output.is_empty() && *killed_clone_1.lock().unwrap() {
                        break;
                    }
                    if !output.is_empty() {
                        output = format!("{} {}", log_time(), output);
                        print!("{output}");
                        let _ = stdout_tx.send(output);
                    }
                }
            })
            .expect("Internal Thread Error: Failed to Spawn [thread:stdoutreader]")
        ;
        let rx_thread = thread::Builder::new()
            .name("stdoutreciever".to_string())
            .spawn(move || {
                let out_buf = Arc::clone(&out_buf);
                let out_buf_lines = Arc::clone(&out_buf_lines);
                loop {
                    match stdout_rx.recv() {
                        Ok(msg) => {
                            if let Err(e) = out_buf.lock().unwrap().write(msg.as_bytes()) {
                                log(LogType::Err, format!("IO Error: Failed To Write To File Buffer: {}", e).as_str());
                                continue
                            }
                            *out_buf_lines.lock().unwrap() += 1;
                        },
                        Err(_) => {
                            log(LogType::Info, "Finished");
                            break
                        }
                    }
                }
            })
            .expect("Internal Thread Error: Failed to Spawn [thread:stdoutreciever]")
        ;

        let flusher_thread = thread::Builder::new()
            .name("stdoutflusher".to_string())
            .spawn(move || {
                let out_buf = Arc::clone(&out_buf_clone);
                let out_buf_lines = Arc::clone(&out_buf_lines_clone);
                loop {
                    if *killed_clone_2.lock().unwrap() {
                        log(LogType::Info, "Finished");
                        break;
                    }
                    let mut out_buf = out_buf.lock().unwrap();
                    let mut out_buf_lines = out_buf_lines.lock().unwrap();
                    if *out_buf_lines > 5 {
                        out_buf.flush().expect("Internal IO Error: Failed To Flush Log File BufWriter in [thread:stdoutflusher]");
                        *out_buf_lines = 0;
                    }
                }
            })
            .expect("Internal Thread Error: Failed to Spawn [thread:stdoutflusher]")
        ;

        Self { 
            tx_thread,
            rx_thread,
            flusher_thread,
            new_stdout_tx,
            killed
        }
    }
    pub fn archive_log(log_file: File) {
        if fs::metadata("logs/archives").is_err() {
            fs::create_dir("logs/archives").expect("FS Error: Failed To Create archives Directory");
        }

        let archives = read_dir("logs/archives").expect("Internal Error: Error Opening Log Archives Folder");

        let mut files = Vec::<DirFile>::new();

        for file in archives.into_iter() {
            match file {
                Ok(file) => {
                    match file.file_name().into_string() {
                        Ok(name) => {
                            if !name.contains("log") {
                                log(LogType::Warn, format!("Incompatable File Found In Archives: {}", &name).as_str());
                                continue;
                            }
                            files.push(DirFile);
                        },
                        Err(e) => log(LogType::Err, format!("Error Converting {:?} To String For Internal Use", e).as_str()) 
                    }
                },
                Err(_) => {
                    log(LogType::Warn, "There Was A Problem Acessing A Log Archive File")
                }
            }
        }

        if files.len() > 15 {
            log(LogType::Warn, "Log Archive Has Reached The Maximum of 15. Delete Some To Remove This Warning");
            return;
        }

        let sys_time = Local::now().naive_local(); 
        let formatted_path = format!("logs/archives/log {}-{} {}:{}", sys_time.month(), sys_time.day(), sys_time.hour(), sys_time.minute());

        let mut new_file = File::create(formatted_path).expect("FS Error: Failed To Create New Archive Log File");

        let old_buf = BufReader::new(log_file);

        new_file.write_all(old_buf.buffer()).expect("IO Error: Failed To Write Old Log Data To New Archive Log Buffer");
        new_file.flush().expect("IO Error: Failed To Flush Old Log Data To New Archive Log File");

    }
    pub fn new_stdout(&mut self, process_stdout: ProcessStdout) {
       let _ = self.new_stdout_tx.send(process_stdout); 
    }
}

impl Kill for LogFile {
    fn kill(self) {
        *self.killed.lock().unwrap() = true;
        self.flusher_thread.join().unwrap();
        self.rx_thread.join().unwrap();
        self.tx_thread.join().unwrap();
    }
}

pub struct DirFile;

pub enum LogType {
    Warn,
    Info,
    Err
}

impl LogType {
    pub fn prefix(&self) -> String {
        let curr_thread = thread::current();
        let thread_name = curr_thread.name().unwrap_or("unamed");
        match self {
            LogType::Warn => format!("[thread:{}:WARN]:", thread_name),
            LogType::Info => format!("[thread:{}:INFO]:", thread_name),
            LogType::Err => format!("[thread:{}:ERR]:", thread_name),
        }
    } 
}

pub fn log_time() -> String {
    let curr_time = Local::now();
    format!("[{}]:", curr_time.format("%m/%d/%y %H:%M:%S"))
}

pub fn log(log_type: LogType, msg: &str) {
    println!("{} {} {}", log_time(), log_type.prefix(), msg);
}
