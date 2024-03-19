use std::{fmt, io::Read};

#[derive(Debug, Clone)]
pub enum CompactSize {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl fmt::Display for CompactSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompactSize::U8(n) => write!(f, "{}", n),
            CompactSize::U16(n) => write!(f, "{}", n),
            CompactSize::U32(n) => write!(f, "{}", n),
            CompactSize::U64(n) => write!(f, "{}", n),
        }
    }
}

impl CompactSize {
    pub fn read_from(stream: &mut dyn Read) -> Result<CompactSize, String> {
        let mut first_byte = [0u8];
        if stream.read_exact(&mut first_byte).is_err() {
            return Err("The stream's format is incorrect".to_string());
        }

        match &first_byte[0] {
            0..=252 => Ok(CompactSize::U8(first_byte[0])),
            253 => {
                let mut two = [0u8; 2];
                if stream.read_exact(&mut two).is_err() {
                    return Err("The stream's format is incorrect".to_string());
                }
                Ok(CompactSize::U16(u16::from_le_bytes(two)))
            }
            254 => {
                let mut four = [0u8; 4];
                if stream.read_exact(&mut four).is_err() {
                    return Err("The stream's format is incorrect".to_string());
                }
                Ok(CompactSize::U32(u32::from_le_bytes(four)))
            }
            255 => {
                let mut eight = [0u8; 8];
                if stream.read_exact(&mut eight).is_err() {
                    return Err("The stream's format is incorrect".to_string());
                }
                Ok(CompactSize::U64(u64::from_le_bytes(eight)))
            }
        }
    }

    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        match self {
            CompactSize::U8(i) => bytes.extend_from_slice(&i.to_be_bytes()),
            CompactSize::U16(i) => {
                bytes.push(253u8);
                bytes.extend_from_slice(&i.to_be_bytes());
            }
            CompactSize::U32(i) => {
                bytes.push(254u8);
                bytes.extend_from_slice(&i.to_be_bytes());
            }
            CompactSize::U64(i) => {
                bytes.push(255u8);
                bytes.extend_from_slice(&i.to_be_bytes());
            }
        };
        bytes
    }

    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        match self {
            CompactSize::U8(i) => bytes.extend_from_slice(&i.to_le_bytes()),
            CompactSize::U16(i) => {
                bytes.push(253u8);
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            CompactSize::U32(i) => {
                bytes.push(254u8);
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            CompactSize::U64(i) => {
                bytes.push(255u8);
                bytes.extend_from_slice(&i.to_le_bytes());
            }
        };
        bytes
    }

    pub fn into_inner(&self) -> usize {
        match self {
            CompactSize::U8(i) => *i as usize,
            CompactSize::U16(i) => *i as usize,
            CompactSize::U32(i) => *i as usize,
            CompactSize::U64(i) => *i as usize,
        }
    }

    pub fn new_from_usize(n: usize) -> CompactSize {
        if n < u8::max_value() as usize {
            return CompactSize::U8(n as u8);
        } else if n < u16::max_value() as usize {
            return CompactSize::U16(n as u16);
        } else if n < u32::max_value() as usize {
            return CompactSize::U32(n as u32);
        }
        CompactSize::U64(n as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_be_bytes() {
        let cs = CompactSize::U16(123);

        assert_eq!(cs.to_be_bytes()[2], 0x7b);

        assert_eq!(cs.to_be_bytes()[1], 0x00);

        assert_eq!(cs.to_be_bytes()[0], 253);
    }

    #[test]
    fn test_to_le_bytes() {
        let cs = CompactSize::U16(123);

        assert_eq!(cs.to_be_bytes()[0], 253);

        assert_eq!(cs.to_be_bytes()[1], 0x00);

        assert_eq!(cs.to_be_bytes()[2], 0x7b);
    }

    #[test]
    fn test_into_inner() {
        let compact_size = CompactSize::U32(123);
        let inner_value = compact_size.into_inner();
        assert_eq!(inner_value, 123);
    }

    #[test]
    fn test_to_string() {
        let compact_size_u8 = CompactSize::U8(42);
        assert_eq!(compact_size_u8.to_string(), "42".to_string());

        let compact_size_u16 = CompactSize::U16(65535);
        assert_eq!(compact_size_u16.to_string(), "65535".to_string());

        let compact_size_u32 = CompactSize::U32(4294967295);
        assert_eq!(compact_size_u32.to_string(), "4294967295".to_string());

        let compact_size_u64 = CompactSize::U64(18446744073709551615);
        assert_eq!(
            compact_size_u64.to_string(),
            "18446744073709551615".to_string()
        );
    }
}
