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

use crate::{process::ProcessStdout, Kill};

const MB: usize = 1024 * 1024;
// const GB: usize = 1024 * 1000000;

pub struct LogFile {
    logger_thread: JoinHandle<()>,
    new_stdout_tx: Sender<ProcessStdout>,
    killed: Arc<Mutex<bool>>,
}

impl LogFile {
    pub fn new(stdout: ProcessStdout, max_file_size: usize) -> Self {
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
        let killed = Arc::new(Mutex::new(false));
        let killed_clone = Arc::clone(&killed);

        let mut out_buf = BufWriter::new(out_file.try_clone().unwrap());

        let (new_stdout_tx, new_stdout_rx) = mpsc::channel::<ProcessStdout>();

        let logger_thread = thread::Builder::new()
            .name("stdoutreader".to_string())
            .spawn(move || {
                let mut stdout_buf = stdout.0;
                let mut out_buf_lines = 0;
                let mut file_size = 0;
                let mut displayed_warning = false;
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
                    if output.is_empty() && *killed_clone.lock().unwrap() {
                        break;
                    }
                    if !output.is_empty() {
                        file_size += output.as_bytes().len();
                        if file_size < max_file_size {
                            output = format!("{} {}", log_time(), output);
                            print!("{output}");
                            if let Err(e) = out_buf.write(output.as_bytes()) {
                                log(LogType::Err, format!("IO Error: Failed To Write To File Buffer: {}", e).as_str());
                                continue
                            }
                            out_buf_lines += 1;
                            if out_buf_lines > 5 {
                                out_buf.flush().expect("Internal FS Error: Failed To Flush Log File BufWriter in [thread:stdoutflusher]");
                                out_buf_lines = 0;
                            }
                        }
                        else if !displayed_warning {
                            log(LogType::Warn, "Max File Size Reached For log.txt");
                            out_buf.flush().expect("Internal FS Error: Failed To Flush Log File BufWriter in [thread:stdoutflusher]");
                            displayed_warning = true;
                        }
                    }
                }
            })
            .expect("Internal Thread Error: Failed to Spawn [thread:stdoutreader]")
        ;

        Self {
            logger_thread,
            new_stdout_tx,
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
    pub fn new_stdout(&mut self, process_stdout: ProcessStdout) {
        let _ = self.new_stdout_tx.send(process_stdout);
    }
}

impl Kill for LogFile {
    fn kill(self) {
        *self.killed.lock().unwrap() = true;
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
