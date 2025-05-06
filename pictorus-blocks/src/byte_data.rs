extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ByteDataError {
    FindByteError,
    UnpackError,
    PackError,
    StartDelimiterNotFound,
    EndDelimiterNotFound,
    InsufficientData,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::EnumString)]
#[strum(ascii_case_insensitive)]
pub enum DataType {
    U8,
    I8,
    U16,
    I16,
    U24,
    I24,
    U32,
    I32,
    U48,
    I48,
    U64,
    I64,
    U128,
    I128,
    F32,
    F64,
}

impl DataType {
    pub fn byte_size(&self) -> usize {
        match self {
            DataType::U8 => 1,
            DataType::I8 => 1,
            DataType::U16 => 2,
            DataType::I16 => 2,
            DataType::U24 => 3,
            DataType::I24 => 3,
            DataType::U32 => 4,
            DataType::I32 => 4,
            DataType::U48 => 6,
            DataType::I48 => 6,
            DataType::U64 => 8,
            DataType::I64 => 8,
            DataType::U128 => 16,
            DataType::I128 => 16,
            DataType::F32 => 4,
            DataType::F64 => 8,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::EnumString)]
pub enum ByteOrderSpec {
    BigEndian,
    LittleEndian,
}
pub fn parse_byte_data_spec<S: AsRef<str>>(data: &[S]) -> Vec<(DataType, ByteOrderSpec)> {
    data.iter()
        .map(|d| {
            d.as_ref()
                .split_once(':')
                .expect("Invalid byte data format")
        })
        .map(|(dt, bo)| {
            (
                dt.parse::<DataType>().unwrap(),
                bo.parse::<ByteOrderSpec>().unwrap(),
            )
        })
        .collect()
}

pub fn compare_bytes(data: &[u8], check: &[u8], skip_byte_indices: &[usize]) -> bool {
    if skip_byte_indices.len() + check.len() != data.len() {
        return false;
    }

    if skip_byte_indices.is_empty() {
        return data == check;
    }

    let mut skipped = 0;
    for (i, c) in data.iter().enumerate() {
        if skip_byte_indices.contains(&i) {
            skipped += 1;
            continue;
        }

        let check_val = &check[i - skipped];
        if c != check_val {
            return false;
        }
    }

    true
}

pub fn find_all_bytes_idx(data: &[u8], check: &[u8], skip_byte_indices: &[usize]) -> Vec<usize> {
    let window_size = check.len() + skip_byte_indices.len();
    data.windows(window_size)
        .enumerate()
        .filter(|&(_, w)| compare_bytes(w, check, skip_byte_indices))
        .map(|(idx, _)| idx)
        .collect()
}

pub fn rfind_all_bytes_idx(data: &[u8], check: &[u8], skip_byte_indices: &[usize]) -> Vec<usize> {
    let window_size = check.len() + skip_byte_indices.len();
    data.windows(window_size)
        .rev()
        .enumerate()
        .filter(|&(_, w)| compare_bytes(w, check, skip_byte_indices))
        .map(|(idx, _)| data.len() - idx - window_size)
        .collect()
}

pub fn find_bytes_idx(
    data: &[u8],
    check: &[u8],
    skip_byte_indices: &[usize],
) -> Result<usize, ByteDataError> {
    // Takes the data and splits it into window sized chunks and
    // searches forwards through the windows for the "check" bytes and then stops
    // at the first match. Then maps the data to index.
    let window_size = check.len() + skip_byte_indices.len();
    data.windows(window_size)
        .enumerate()
        .find(|&(_, w)| compare_bytes(w, check, skip_byte_indices))
        .map(|(idx, _)| idx)
        .ok_or(ByteDataError::FindByteError)
}

pub fn rfind_bytes_idx(
    data: &[u8],
    check: &[u8],
    skip_byte_indices: &[usize],
) -> Result<usize, ByteDataError> {
    let window_size = check.len() + skip_byte_indices.len();
    // Takes the data and splits it into window sized chunks and
    // searches backwards through the windows for the "check" bytes and then stops
    // at the first match. Then maps the data to index accounting for the window size.
    data.windows(window_size)
        .rev()
        .enumerate()
        .find(|&(_, w)| compare_bytes(w, check, skip_byte_indices))
        .map(|(idx, _)| data.len() - idx - window_size)
        .ok_or(ByteDataError::FindByteError)
}

const HEX_DELIM: &str = r"\x";
pub fn parse_string_to_bytes(data: &str) -> Vec<u8> {
    if data.starts_with(HEX_DELIM) {
        let split_data: Vec<&str> = data.split(HEX_DELIM).filter(|c| !c.is_empty()).collect();
        let byte_data: Vec<u8> = split_data
            .iter()
            .filter_map(|c| u8::from_str_radix(c, 16).ok())
            .collect();

        if byte_data.len() == split_data.len() {
            return byte_data;
        }
    }

    Vec::from(data.as_bytes())
}

pub fn parse_string_to_read_delimiter(data: &str) -> (Vec<u8>, Vec<usize>) {
    if data.starts_with(HEX_DELIM) {
        let split_data: Vec<&str> = data.split(HEX_DELIM).filter(|c| !c.is_empty()).collect();
        let byte_data: Vec<u8> = split_data
            .iter()
            .filter_map(|c| u8::from_str_radix(c, 16).ok())
            .collect();

        let skip_bytes: Vec<usize> = split_data
            .iter()
            .enumerate()
            .filter_map(|(i, c)| if *c == "**" { Some(i) } else { None })
            .collect();

        if byte_data.len() + skip_bytes.len() == split_data.len() {
            return (byte_data, skip_bytes);
        }
    }

    (Vec::from(data.as_bytes()), Vec::new())
}

pub const BUFF_SIZE_BYTES: usize = 1024;

use byteorder::ByteOrder;

pub fn try_unpack_data<Endian: ByteOrder>(
    buf: &[u8],
    data_type: DataType,
) -> Result<f64, ByteDataError> {
    if buf.len() < data_type.byte_size() {
        return Err(ByteDataError::UnpackError);
    }
    let val = match data_type {
        DataType::U8 => buf[0] as f64,
        DataType::I8 => buf[0] as i8 as f64,
        DataType::U16 => Endian::read_u16(buf) as f64,
        DataType::I16 => Endian::read_i16(buf) as f64,
        DataType::U24 => Endian::read_u24(buf) as f64,
        DataType::I24 => Endian::read_i24(buf) as f64,
        DataType::U32 => Endian::read_u32(buf) as f64,
        DataType::I32 => Endian::read_i32(buf) as f64,
        DataType::U48 => Endian::read_u48(buf) as f64,
        DataType::I48 => Endian::read_i48(buf) as f64,
        DataType::U64 => Endian::read_u64(buf) as f64,
        DataType::I64 => Endian::read_i64(buf) as f64,
        DataType::U128 => Endian::read_u128(buf) as f64,
        DataType::I128 => Endian::read_i128(buf) as f64,
        DataType::F32 => Endian::read_f32(buf) as f64,
        DataType::F64 => Endian::read_f64(buf),
    };
    Ok(val)
}

pub fn try_pack_data<Endian: ByteOrder>(
    buf: &mut [u8],
    value: f64,
    data_type: DataType,
) -> Result<usize, ByteDataError> {
    if buf.len() < data_type.byte_size() {
        return Err(ByteDataError::PackError);
    }
    match data_type {
        DataType::U8 => buf[0] = value as u8,
        DataType::I8 => buf[0] = value as i8 as u8,
        DataType::U16 => Endian::write_u16(buf, value as u16),
        DataType::I16 => Endian::write_i16(buf, value as i16),
        DataType::U24 => Endian::write_u24(buf, value as u32),
        DataType::I24 => Endian::write_i24(buf, value as i32),
        DataType::U32 => Endian::write_u32(buf, value as u32),
        DataType::I32 => Endian::write_i32(buf, value as i32),
        DataType::U48 => Endian::write_u48(buf, value as u64),
        DataType::I48 => Endian::write_i48(buf, value as i64),
        DataType::U64 => Endian::write_u64(buf, value as u64),
        DataType::I64 => Endian::write_i64(buf, value as i64),
        DataType::U128 => Endian::write_u128(buf, value as u128),
        DataType::I128 => Endian::write_i128(buf, value as i128),
        DataType::F32 => Endian::write_f32(buf, value as f32),
        DataType::F64 => Endian::write_f64(buf, value),
    };
    Ok(data_type.byte_size())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;
    use byteorder::BigEndian;

    #[test]
    fn test_parse_byte_data_spec() {
        let input = vec![
            "U8:BigEndian".to_string(),
            "I16:LittleEndian".to_string(),
            "F32:BigEndian".to_string(),
        ];

        let expected = vec![
            (DataType::U8, ByteOrderSpec::BigEndian),
            (DataType::I16, ByteOrderSpec::LittleEndian),
            (DataType::F32, ByteOrderSpec::BigEndian),
        ];

        let output = parse_byte_data_spec(&input);

        assert_eq!(output, expected);
    }

    #[test]
    fn test_compare_bytes() {
        let result = compare_bytes(b"Hello, world!", b"Hello, world!", &[]);
        assert!(result);

        let result = compare_bytes(b"Hello, world!", b"Hello, wor", &[]);
        assert!(!result);

        let result = compare_bytes(b"012345", b"045", &[1, 2, 3]);
        assert!(result);

        let result = compare_bytes(b"012345", b"0c45", &[1, 2]);
        assert!(!result);
    }

    #[test]
    fn test_find_all_bytes_idx() {
        let data = b"Hello, world! This is an example of finding bytes.";
        let pattern = b"l";
        let expected = vec![2, 3, 10, 30];
        let result = find_all_bytes_idx(data, pattern, &[]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_all_bytes_idx_skip_indices() {
        let data = b"aaa123aba123aca123";
        let pattern = b"aa";
        let expected = vec![0, 6, 12];
        let result = find_all_bytes_idx(data, pattern, &[1]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rfind_all_bytes_idx() {
        let data = b"Hello, world! This is an example of finding bytes.";
        let pattern = b"l";
        let expected = vec![30, 10, 3, 2];
        let result = rfind_all_bytes_idx(data, pattern, &[]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rfind_all_bytes_idx_skip_indices() {
        let data = b"aaa123aba123aca123";
        let pattern = b"aa";
        let expected = vec![12, 6, 0];
        let result = rfind_all_bytes_idx(data, pattern, &[1]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_bytes_idx() {
        let data = b"Hello, world! This is an example of finding bytes.";
        let pattern = b"world";
        let expected = Ok(7);
        let result = find_bytes_idx(data, pattern, &[]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_bytes_idx_skip_indices() {
        let data = b"aaa123aba123aca123";
        let pattern = b"aa";
        let expected = Ok(0);
        let result = find_bytes_idx(data, pattern, &[1]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rfind_bytes_idx() {
        let data = b"Hello, world! This is an example of finding bytes.";
        let pattern = b"world";
        let expected = Ok(7);
        let result = rfind_bytes_idx(data, pattern, &[]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rfind_bytes_idx_skip_indices() {
        let data = b"aaa123aba123aca123";
        let pattern = b"aa";
        let expected = Ok(12);
        let result = rfind_bytes_idx(data, pattern, &[1]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_string_to_bytes() {
        let input = r"\x48\x65\x6C\x6C\x6F\x2C\x20\x77\x6F\x72\x6C\x64\x21";
        let expected = b"Hello, world!".to_vec();
        let result = parse_string_to_bytes(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_string_to_read_delimiter() {
        let input = r"\x48\x65\x**\x6c\x**\x2C";
        let expected = (b"\x48\x65\x6c\x2C".to_vec(), vec![2, 4]);
        let result = parse_string_to_read_delimiter(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_try_unpack_data_and_try_pack_data() {
        let input: f64 = 1234.5678;
        let dt = DataType::F64;
        let mut packed_data = vec![0; dt.byte_size()];

        try_pack_data::<BigEndian>(&mut packed_data, input, dt).unwrap();
        let unpacked_data = try_unpack_data::<BigEndian>(&packed_data, dt).unwrap();

        assert_eq!(input, unpacked_data);
    }
}
