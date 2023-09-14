use std::{
    fs::{self, create_dir, read_dir, remove_file, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::PathBuf,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::{Local, NaiveDateTime};

use crate::{process::ProcessOutput, Kill};

const MB: usize = 1024 * 1024;
// const GB: usize = 1024 * 1000000;

pub enum KillType {
    Crash,
    Kill,
}

pub struct LogFile {
    logger_thread: JoinHandle<()>,
    new_out_tx: Sender<ProcessOutput>,
    // client_log_tx: Sender<String>,
    killed: Arc<Mutex<Option<KillType>>>,
}

impl LogFile {
    pub fn new(process_output: ProcessOutput, max_file_size: usize) -> Self {
        let mut new_log = true;
        if fs::metadata("logs").is_err() {
            create_dir("logs").expect("FS Error: Failed To Create logs Directory");
        }
        let mut out_file = match OpenOptions::new()
            .write(true)
            .read(true)
            .open("logs/log.txt")
        {
            Ok(file) => {
                new_log = false;
                file
            }
            Err(_) => {
                File::create("logs/log.txt").expect("Internal Error: Error Creating New Log File")
            }
        };

        if !new_log {
            out_file = LogFile::archive_log(out_file.try_clone().unwrap(), max_file_size);
        }
        let killed = Arc::new(Mutex::new(None));
        let killed_clone = Arc::clone(&killed);

        let mut out_buf = BufWriter::new(out_file.try_clone().unwrap());

        let (new_out_tx, new_out_rx) = mpsc::channel::<ProcessOutput>();
        // let (client_log_tx, client_log_rx) = mpsc::channel::<String>();

        let logger_thread = thread::Builder::new()
            .name("loggerthread".to_string())
            .spawn(move || {
                let mut stdout_buf = process_output.stdout;
                let mut stderr_buf = process_output.stderr;
                let mut file_size = 0;
                let mut displayed_warning = false;
                // Output is an Option where Some means you want to log and output whereas None
                // means you just want to flush the buffer
                let mut log_output = |output: Option<String>| {
                    if let Some(output) = output {
                        if let Err(e) = out_buf.write(output.as_bytes()) {
                            log(LogType::Err, format!("IO Error: Failed To Write To File Buffer: {}", e).as_str());
                        }
                        if let Err(e) = out_buf.flush() {
                            log(LogType::Err, format!("Internal FS Error: Failed To Flush Log File BufWriter To Filesystem. {}", e).as_str());
                        }
                    }
                    else {
                        if let Err(e) = out_buf.flush() {
                            log(LogType::Err, format!("Internal FS Error: Failed To Flush Log File BufWriter To Filesystem. {}", e).as_str());
                        }
                    }
                };
                loop {
                    if let Ok(new_out) = new_out_rx.recv_timeout(Duration::from_millis(1)) {
                        stdout_buf = new_out.stdout;
                        stderr_buf = new_out.stderr;
                    }
                    if stdout_buf.buffer().len() >= stdout_buf.capacity() {
                        let stdout = stdout_buf.into_inner();
                        stdout_buf = BufReader::new(stdout);
                    }
                    let kill_type = killed_clone.lock().unwrap().take();
                    let mut std_out_output = String::new();
                    stdout_buf.read_line(&mut std_out_output).expect("Internal IO Error: Error Reading Line From Child Process stdout");
                    if let Some(kill_type) = kill_type {
                        match kill_type {
                            KillType::Crash => {
                                thread::sleep(Duration::from_millis(10));
                                loop {
                                    let mut std_err_output = String::new();
                                    stderr_buf.read_line(&mut std_err_output).expect("Internal IO Error: Error Reading Line From Child Process stderr");
                                    if std_err_output.is_empty() {
                                        *killed_clone.lock().unwrap() = None;
                                        break;
                                    }
                                    std_err_output = format!("{} {}", log_time(), std_err_output); 
                                    print!("{std_err_output}");
                                    log_output(Some(std_err_output));
                                }
                            } 
                            KillType::Kill => break
                        }
                    }
                    if !std_out_output.is_empty() {
                        file_size += std_out_output.as_bytes().len();
                        if file_size < max_file_size {
                            std_out_output= format!("{} {}", log_time(), std_out_output);
                            print!("{std_out_output}");
                            log_output(Some(std_out_output));
                        }
                        else if !displayed_warning {
                            log(LogType::Warn, "Max File Size Reached For log.txt");
                            log_output(None);
                            displayed_warning = true;
                        }
                    }
                }
            })
            .expect("Internal Thread Error: Failed to Spawn [thread:loggerthread]")
        ;

        Self {
            logger_thread,
            // stderr_finished,
            new_out_tx,
            // client_log_tx,
            killed,
        }
    }
    pub fn archive_log(mut log_file: File, max_file_size: usize) -> File {
        let date_format = "%m-%d-%y %H:%M";
        log(LogType::Info, "Creating New Archived Log File");
        if fs::metadata("logs/archives").is_err() {
            fs::create_dir("logs/archives").expect("FS Error: Failed To Create archives Directory");
        }

        let archives =
            read_dir("logs/archives").expect("Internal Error: Error Opening Log Archives Folder");

        let mut files = Vec::<DirFile>::new();

        for file in archives.into_iter() {
            match file {
                Ok(file) => match file.file_name().into_string() {
                    Ok(name) => {
                        if name.contains(".DS_Store") {
                            continue;
                        } else if !name.contains("log") {
                            log(
                                LogType::Warn,
                                format!("Incompatable File Found In Archives: {}", &name).as_str(),
                            );
                            continue;
                        }
                        let date = match NaiveDateTime::parse_from_str(
                            name.replace("log ", "").as_str(),
                            date_format,
                        ) {
                            Ok(date) => date,
                            Err(_) => Local::now().naive_local(),
                        };
                        files.push(DirFile {
                            date_of_creation: date,
                            path: file.path(),
                            name,
                        });
                    }
                    Err(e) => log(
                        LogType::Err,
                        format!("Error Converting {:?} To String For Internal Use", e).as_str(),
                    ),
                },
                Err(_) => log(
                    LogType::Warn,
                    "There Was A Problem Acessing A Log Archive File",
                ),
            }
        }

        if files.len() > 15 {
            let oldest_file_index = LogFile::find_oldest_file_date(&files).unwrap();
            let oldest_file = files.get(oldest_file_index).unwrap();

            match remove_file(oldest_file.path.clone()) {
                Ok(_) =>  log(
                    LogType::Info,
                    format!(
                        "Log Archive Has Reached A Maximum of 15 Files. The Oldest File Was Removed `{}`.\nMove Files To Your Own Directory Manually To Prevent The Automatic Removal Of The Oldest File In The Future", oldest_file.name).as_str()
                    ),
                Err(_) => log(
                    LogType::Warn,
                    "Log Archive Has Reached The Maximum of 15. And An Error Occurred While Trying To Delete The Oldest File. Delete Some To Remove This Warning",
                )
            }

            return File::create("logs/log.txt")
                .expect("FS Error: Failed To Create New log.txt File");
        }

        let sys_time = Local::now().format(date_format);
        let formatted_path = format!("logs/archives/log {}", sys_time);

        let mut new_file =
            File::create(formatted_path).expect("FS Error: Failed To Create New Archive Log File");
        let file_size = match log_file.metadata() {
            Ok(metadata) => metadata.len() as usize,
            Err(e) => {
                log(LogType::Err, format!("Failed To Aquire MetaData for log.txt. Defaulting File Size To Config 'max_file_size' of {}. {}", max_file_size, e).as_str());
                max_file_size
            }
        };

        log(
            LogType::Info,
            format!(
                "Preparing To Copy Old Log File To New Log File With Size {}",
                file_size
            )
            .as_str(),
        );
        let mut buf_len = std::cmp::min(MB, file_size);
        loop {
            let mut old_buf = vec![0; buf_len];
            match log_file.read(&mut old_buf) {
                Ok(n) => {
                    new_file
                        .write_all(&old_buf[0..n])
                        .expect("IO Error: Failed To Write Old Log Data To New Archive Log Buffer");
                    if n == 0 {
                        log(
                            LogType::Info,
                            format!("All {} Bytes Read From Old Log File", file_size).as_str(),
                        );
                        break;
                    }
                }
                Err(e) => {
                    log(
                        LogType::Err,
                        format!("Error Reading From Old Log File Into New Buffer {:?}", e).as_str(),
                    );
                    break;
                }
            }

            // If we read less than the buffer size, we reached the end of the file
            if buf_len > file_size {
                break;
            }

            // Decrease the buffer size for the next iteration, but keep it at least 1 byte
            buf_len = std::cmp::max(1, buf_len / 2);
        }
        File::create("logs/log.txt").expect("FS Error: Failed To Create New log.txt File")
    }
    fn find_oldest_file_date(files: &[DirFile]) -> Option<usize> {
        if files.is_empty() {
            return None;
        }

        let mut oldest_date = files[0].date_of_creation;
        let mut file_to_remove = 0;
        for (i, file) in files.iter().enumerate() {
            let file_date = file.date_of_creation.clone();
            if file_date < oldest_date {
                oldest_date = file_date.clone();
                file_to_remove = i;
            }
        }
        Some(file_to_remove)
    }
    pub fn new_process_out(&mut self, process_output: ProcessOutput) {
        let _ = self.new_out_tx.send(process_output);
    }
    pub fn report_crash(&mut self) {
        *self.killed.lock().unwrap() = Some(KillType::Crash);
    }
    // pub fn log_from_client(&mut self, output: String) {
    //     let _ = self.client_log_tx.send(output);
    // }
}

impl Kill for LogFile {
    fn kill(self) {
        *self.killed.lock().unwrap() = Some(KillType::Kill);
        self.logger_thread.join().unwrap();
    }
}

pub struct DirFile {
    pub date_of_creation: NaiveDateTime,
    pub path: PathBuf,
    pub name: String,
}

pub enum LogType {
    Warn,
    Info,
    Err,
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
