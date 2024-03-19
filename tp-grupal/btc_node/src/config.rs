use std::{
    error::Error,
    fmt,
    fs::File,
    io::{BufRead, BufReader},
    time::Duration,
};

#[derive(Debug)]
pub enum ConfigError {
    ConfigFileError(std::io::Error),
    MissingFieldError(String),
    ParsingError(String),
}

impl Error for ConfigError {}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::ConfigFileError(error) => {
                write!(
                    f,
                    "An error occurred while opening the configuration file: {}",
                    error
                )
            }
            ConfigError::MissingFieldError(field) => {
                write!(f, "Missing field in the configuration: {}", field)
            }
            ConfigError::ParsingError(field) => {
                write!(f, "Error ocurred while parsing: {}", field)
            }
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        ConfigError::ConfigFileError(error)
    }
}

pub struct ConfigBuilder {
    dns: Option<String>,
    port: Option<u16>,
    tcp_timeout: Option<Duration>,
    blockchain_file: Option<String>,
    log_file: Option<String>,
    block_downloading_timestamp: Option<u32>,
    block_downloading_threads: Option<usize>,
    max_listen_peers: Option<usize>,
    host: Option<String>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder {
            dns: None,
            port: None,
            tcp_timeout: None,
            blockchain_file: None,
            log_file: None,
            block_downloading_timestamp: None,
            block_downloading_threads: None,
            max_listen_peers: None,
            host: None,
        }
    }

    pub fn block_downloading_threads(mut self, block_downloading_threads: usize) -> ConfigBuilder {
        self.block_downloading_threads = Some(block_downloading_threads);
        self
    }

    pub fn max_listen_peers(mut self, max_listen_peers: usize) -> ConfigBuilder {
        self.max_listen_peers = Some(max_listen_peers);
        self
    }

    pub fn block_downloading_timestamp(
        mut self,
        block_downloading_timestamp: u32,
    ) -> ConfigBuilder {
        self.block_downloading_timestamp = Some(block_downloading_timestamp);
        self
    }

    pub fn blockchain_file(mut self, blockchain_file: String) -> ConfigBuilder {
        self.blockchain_file = Some(blockchain_file);
        self
    }

    pub fn log_file(mut self, log_file: String) -> ConfigBuilder {
        self.log_file = Some(log_file);
        self
    }

    pub fn dns(mut self, dns: String) -> ConfigBuilder {
        self.dns = Some(dns);
        self
    }

    fn port(mut self, port: u16) -> ConfigBuilder {
        self.port = Some(port);
        self
    }

    fn tcp_timeout(mut self, tcp_timeout: Duration) -> ConfigBuilder {
        self.tcp_timeout = Some(tcp_timeout);
        self
    }

    pub fn host(mut self, host: String) -> ConfigBuilder {
        self.host = Some(host);
        self
    }

    pub fn build(self) -> Result<Config, ConfigError> {
        let endpoint = self
            .dns
            .ok_or_else(|| ConfigError::MissingFieldError("endpoint".to_string()))?;

        let port = self
            .port
            .ok_or_else(|| ConfigError::MissingFieldError("port".to_string()))?;

        let tcp_timeout = self
            .tcp_timeout
            .ok_or_else(|| ConfigError::MissingFieldError("tcp_timeout".to_string()))?;

        let blockchain_file = self
            .blockchain_file
            .ok_or_else(|| ConfigError::MissingFieldError("tcp_timeout".to_string()))?;

        let log_file = self
            .log_file
            .ok_or_else(|| ConfigError::MissingFieldError("tcp_timeout".to_string()))?;

        let block_downloading_timestamp = self.block_downloading_timestamp.ok_or_else(|| {
            ConfigError::MissingFieldError("block_downloading_timestamp".to_string())
        })?;

        let block_downloading_threads = self.block_downloading_threads.ok_or_else(|| {
            ConfigError::MissingFieldError("block_downloading_threads".to_string())
        })?;

        let max_listen_peers = self
            .max_listen_peers
            .ok_or_else(|| ConfigError::MissingFieldError("max_listen_peers".to_string()))?;

        Ok(Config {
            endpoint,
            port,
            tcp_timeout,
            blockchain_file,
            log_file,
            block_downloading_timestamp,
            block_downloading_threads,
            max_listen_peers,
            host: self.host,
        })
    }
}

#[derive(Debug)]
pub struct Config {
    pub endpoint: String,
    pub port: u16,
    pub tcp_timeout: Duration,
    pub blockchain_file: String,
    pub log_file: String,
    pub block_downloading_timestamp: u32,
    pub block_downloading_threads: usize,
    pub max_listen_peers: usize,
    pub host: Option<String>,
}

const SEPARATOR: char = '=';

impl Config {
    pub fn new(config_file_path: &String) -> Result<Config, ConfigError> {
        let mut builder = ConfigBuilder::new();
        let file = File::open(config_file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?.to_lowercase();
            let parts: Vec<&str> = line.splitn(2, SEPARATOR).collect();

            if parts.len() < 2 {
                continue;
            }

            let value = match parts[1].split_whitespace().next() {
                None => continue,
                Some(i) => i,
            };

            builder = match parts[0] {
                "dns" => builder.dns(value.to_string()),
                "port" => {
                    let port = u16::from_str_radix(value, 10)
                        .map_err(|_| ConfigError::ParsingError("port".to_string()))?;
                    builder.port(port)
                }
                "tcp_timeout" => {
                    let duration = u64::from_str_radix(value, 10)
                        .map_err(|_| ConfigError::ParsingError("tcp_timeout".to_string()))?;
                    builder.tcp_timeout(Duration::from_secs(duration))
                }
                "blockchain_file" => builder.blockchain_file(value.to_string()),
                "log_file" => builder.log_file(value.to_string()),
                "block_downloading_timestamp" => {
                    let timestamp = u32::from_str_radix(value, 10).map_err(|_| {
                        ConfigError::ParsingError("block_downloading_timestamp".to_string())
                    })?;
                    builder.block_downloading_timestamp(timestamp)
                }
                "block_downloading_threads" => {
                    let threads = usize::from_str_radix(value, 10).map_err(|_| {
                        ConfigError::ParsingError("block_downloading_threads".to_string())
                    })?;
                    builder.block_downloading_threads(threads)
                }
                "max_listen_peers" => {
                    let peers = usize::from_str_radix(value, 10)
                        .map_err(|_| ConfigError::ParsingError("max_listen_peers".to_string()))?;
                    builder.max_listen_peers(peers)
                }
                "host" => builder.host(value.to_string()),
                _ => {
                    continue;
                }
            }
        }

        builder.build()
    }
}
