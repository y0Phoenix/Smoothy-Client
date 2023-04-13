use std::{io::{BufWriter, Stdout, Write, BufReader, BufRead}, fs::{File, OpenOptions, read_dir, FileType, ReadDir, self, create_dir}, thread::{JoinHandle, self}, sync::{mpsc::{Receiver, self, Sender}, Arc, Mutex}, process::ChildStdout, time::Duration};

use chrono::{Local, Timelike, Datelike};

use crate::{process::ProcessStdout, Restart, Kill};

pub struct LogFile {
    tx_thread: JoinHandle<()>,
    rx_thread: JoinHandle<()>,
    flusher_thread: JoinHandle<()>,
    new_stdout_tx: Sender<ProcessStdout>
}

impl LogFile {
    pub fn new(stdout: ProcessStdout) -> Self {
        let mut new_log = true;
        if !fs::metadata("logs").is_ok() {
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
                    match new_stdout_rx.recv_timeout(Duration::from_secs(1)) {
                        Ok(new_stdout) => stdout_buf = new_stdout.0,
                        _ => {},
                    }
                    if stdout_buf.buffer().len() >= stdout_buf.capacity() {
                        let stdout = stdout_buf.into_inner();
                        stdout_buf = BufReader::new(stdout);
                    }
                    let mut output = String::new();
                    stdout_buf.read_line(&mut output).expect("Internal IO Error: Error Reading Line From Child Process stdout");
                    let curr_time = Local::now();
                    output = format!("[{}]: {}", curr_time.format("%m/%d/%y %H:%M:%S"), output);
                    print!("{}", output);
                    let _ = stdout_tx.send(output);
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
                            match out_buf.lock().unwrap().write(msg.as_bytes()) {
                                Err(e) => {
                                    log(LogType::ERR, format!("IO Error: Failed To Write To File Buffer: {}", e).as_str());
                                    continue
                                },
                                _ => {}
                            }
                            *out_buf_lines.lock().unwrap() += 1;
                        },
                        Err(_) => break
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
            new_stdout_tx
        }
    }
    pub fn archive_log(log_file: File) {
        let archives: ReadDir;
        if !fs::metadata("logs/archives").is_ok() {
            fs::create_dir("logs/archives").expect("FS Error: Failed To Create archives Directory");
        }

        archives = read_dir("logs/archives").expect("Internal Error: Error Opening Log Archives Folder");

        let mut files = Vec::<DirFile>::new();

        for (i, file) in archives.into_iter().enumerate() {
            match file {
                Ok(file) => {
                    if let Ok(file_type) = file.file_type() {
                        match file.file_name().into_string() {
                            Ok(name) => {
                                 if !name.contains("log") {
                                    log(LogType::WARN, format!("Incompatable File Found In Archives: {}", &name).as_str());
                                    continue;
                                }
                                files.push(DirFile { name, file_type });
                            },
                            Err(e) => log(LogType::ERR, format!("Error Converting {:?} To String For Internal Use", e).as_str()) 
                        }
                    }
                },
                Err(_) => {
                    log(LogType::WARN, "There Was A Problem Acessing A Log Archive File")
                }
            }
        }

        if files.len() > 15 {
            log(LogType::WARN, "Log Archive Has Reached The Maximum of 15. Delete Some To Remove This Warning");
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
        self.flusher_thread.join().unwrap();
        self.rx_thread.join().unwrap();
        self.tx_thread.join().unwrap();
    }
}

pub struct DirFile {
    name: String,
    file_type: FileType
}

pub struct ConsoleLog {
    stdout: Stdout
}

impl ConsoleLog {
    fn new() -> Self {
        let stdout = std::io::stdout();
        Self {stdout}
    }
}

pub enum LogType {
    WARN,
    INFO,
    ERR
}

impl LogType {
    pub fn prefix(&self) -> String {
        let curr_thread = thread::current();
        let thread_name = curr_thread.name().unwrap_or("unamed");
        match self {
            LogType::WARN => format!("[thread:{}:WARN] ", thread_name),
            LogType::INFO => format!("[thread:{}:INFO] ", thread_name),
            LogType::ERR => format!("[thread:{}:ERR] ", thread_name),
        }
    } 
}

pub fn log(log_type: LogType, msg: &str) {
    println!("{}{}", log_type.prefix(), msg);
}
