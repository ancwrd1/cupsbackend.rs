use std::{
    collections::HashMap,
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    process::exit,
};

use log::{error, info, LevelFilter};
use tempfile::NamedTempFile;
use url::Url;

const NAME: &str = "testbackend";
const DESCRIPTION: &str = "CUPS backend in Rust";

pub enum JobSource {
    JobFile(PathBuf),
    TempFile(NamedTempFile),
}

impl JobSource {
    pub fn path(&self) -> &Path {
        match self {
            JobSource::JobFile(ref path) => path,
            JobSource::TempFile(ref temp) => temp.path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ExitCode {
    Success,
    ErrorPolicy,
    AuthRequired,
    HoldJob,
    StopQueue,
    CancelJob,
}

#[derive(Debug)]
pub enum BackendError {
    NoArgs,
    BadArgs,
    NoUri,
    IOError(io::Error),
}

impl BackendError {
    pub fn to_exit_code(&self) -> ExitCode {
        match *self {
            BackendError::NoArgs => ExitCode::Success,
            BackendError::BadArgs => ExitCode::ErrorPolicy,
            _ => ExitCode::CancelJob,
        }
    }
}

impl From<io::Error> for BackendError {
    fn from(error: io::Error) -> BackendError {
        BackendError::IOError(error)
    }
}

pub struct BackendData {
    pub printer_uri: Url,
    pub user_name: String,
    pub title: String,
    pub copies: u32,
    pub options: HashMap<String, String>,
    pub job_source: JobSource,
}

pub type Result<T> = std::result::Result<T, BackendError>;

impl BackendData {
    fn parse_args() -> Result<BackendData> {
        let args: Vec<_> = env::args().collect();

        if args.len() < 2 {
            return Err(BackendError::NoArgs);
        } else if args.len() != 6 && args.len() != 7 {
            return Err(BackendError::BadArgs);
        }

        let printer_uri = if let Some(uri) = env::var("DEVICE_URI")
            .ok()
            .and_then(|uri| Url::parse(&uri).ok())
        {
            uri
        } else {
            return Err(BackendError::NoUri);
        };

        let user_name = args[2].clone();

        let title = if !args[3].is_empty() {
            args[3].clone()
        } else if args.len() >= 7 {
            Path::new(&args[6])
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            String::from("untitled")
        };

        let copies = args[4].parse::<u32>().unwrap_or(1);

        let mut options = HashMap::new();

        for opt in args[5].split_whitespace() {
            let mut kv = opt.splitn(2, '=');
            if let Some(k) = kv.next() {
                let v = kv.next().unwrap_or("true");
                options.insert(k.to_lowercase(), v.to_lowercase());
            }
        }

        let job_source = if args.len() >= 7 {
            JobSource::JobFile(PathBuf::from(&args[6]))
        } else {
            let mut tmp = tempfile::NamedTempFile::new()?;
            io::copy(&mut io::stdin(), &mut tmp)?;
            JobSource::TempFile(tmp)
        };

        Ok(BackendData {
            printer_uri,
            user_name,
            title,
            copies,
            options,
            job_source,
        })
    }
}

#[derive(Default)]
pub struct CupsBackend;

impl CupsBackend {
    fn advertise(&self) {
        println!("direct {}:// \"Unknown\" \"{}\"", NAME, DESCRIPTION);
    }

    fn usage(&self) {
        eprintln!(
            "Usage: {} job-id user title copies options [file]",
            env::args().next().unwrap()
        );
    }

    pub fn new() -> CupsBackend {
        CupsBackend::default()
    }

    pub fn run(&self) {
        env::set_var("RUST_LOG", "debug");

        let mut builder = env_logger::builder();
        builder.format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()));
        let _ = log::set_boxed_logger(Box::new(builder.build()));
        log::set_max_level(LevelFilter::Debug);

        let code = match BackendData::parse_args() {
            Ok(data) => self.process_data(data),
            Err(err) => {
                match err {
                    BackendError::NoArgs => self.advertise(),
                    BackendError::BadArgs => self.usage(),
                    BackendError::NoUri => error!("No printer URI"),
                    BackendError::IOError(ref e) => error!("{}", e),
                }
                err.to_exit_code()
            }
        };
        exit(code as i32);
    }

    fn process_data(&self, data: BackendData) -> ExitCode {
        info!("Processing job: {}", data.title);
        ExitCode::Success
    }
}
