use crate::{constants, protocol_error::ProtocolError};
use bitcoin_hashes::{sha256d, Hash};
use std::io::{Read, Write};

pub mod message_header_error {
    use std::error::Error;
    use std::fmt;
    use std::string::FromUtf8Error;

    #[derive(Debug)]
    pub enum MessageHeaderError {
        CommandTooLong(String),
        ChecksumFailed(String),
        IOError(std::io::Error),
        InvalidCommand(FromUtf8Error),
    }

    impl Error for MessageHeaderError {}

    impl fmt::Display for MessageHeaderError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                MessageHeaderError::CommandTooLong(s) => write!(f, "{}", s),
                MessageHeaderError::ChecksumFailed(s) => write!(f, "{}", s),
                MessageHeaderError::IOError(s) => write!(f, "{}", s),
                MessageHeaderError::InvalidCommand(s) => write!(f, "{}", s),
            }
        }
    }

    impl From<std::io::Error> for MessageHeaderError {
        fn from(error: std::io::Error) -> Self {
            MessageHeaderError::IOError(error)
        }
    }

    impl From<FromUtf8Error> for MessageHeaderError {
        fn from(error: FromUtf8Error) -> Self {
            MessageHeaderError::InvalidCommand(error)
        }
    }
}

use message_header_error::MessageHeaderError;

#[derive(Debug)]
pub struct MessageHeader {
    pub start_string: [u8; 4],
    pub command_name: [u8; 12],
    pub payload_size: u32,
    pub checksum: [u8; 4],
}

use std::fmt;

impl fmt::Display for MessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "start_string: {:?}", self.start_string)?;
        writeln!(f, "command_name: {:?}", self.command_name)?;
        writeln!(f, "payload_size: {}", self.payload_size)?;
        writeln!(f, "checksum: {:?}", self.checksum)
    }
}

impl MessageHeader {
    pub fn new(command: String, payload: Vec<u8>) -> Result<MessageHeader, MessageHeaderError> {
        if command.len() > 12 {
            return Err(MessageHeaderError::CommandTooLong(
                "Command too long.".to_string(),
            ));
        }

        let start_string = constants::START_STRING;

        let mut command_name = [0u8; 12];
        command_name[..command.len()].copy_from_slice(command.as_bytes());

        let checksum: [u8; 4];
        if let Ok(checksum_success) = sha256d::Hash::hash(&payload[..])[0..4].try_into() {
            checksum = checksum_success;
        } else {
            return Err(MessageHeaderError::ChecksumFailed(
                "Failed to calculate checksum.".to_string(),
            ));
        }

        let payload_size = payload.len() as u32;

        Ok(MessageHeader {
            start_string,
            command_name,
            payload_size,
            checksum,
        })
    }

    pub fn validate_checksum(&self) -> bool {
        self.checksum == sha256d::Hash::hash(&[]).to_byte_array()[0..4]
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        stream.write_all(&self.start_string)?;
        stream.write_all(&self.command_name)?;
        stream.write_all(&self.payload_size.to_le_bytes())?;
        stream.write_all(&self.checksum)?;
        Ok(())
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<MessageHeader, MessageHeaderError> {
        let mut start_string = [0u8; 4];
        stream.read_exact(&mut start_string)?;

        let mut command_name = [0u8; 12];
        stream.read_exact(&mut command_name)?;

        let mut payload_size_buf = [0u8; 4];
        stream.read_exact(&mut payload_size_buf)?;
        let payload_size = u32::from_le_bytes(payload_size_buf);

        let mut checksum = [0u8; 4];
        stream.read_exact(&mut checksum)?;

        Ok(MessageHeader {
            start_string,
            command_name,
            payload_size,
            checksum,
        })
    }

    pub fn command_name(&self) -> Result<String, MessageHeaderError> {
        let mut s = String::from_utf8(self.command_name.to_vec())?;
        while s.ends_with('\0') {
            s.pop();
        }
        Ok(s)
    }
}
