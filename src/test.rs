
#[cfg(test)]
mod serialised_data_tests {
    use crate::parser::serialised_data::*;
    use crate::datastructures::serialised_data::*;
    #[test]
    fn test_byte_str() {
        let res1: Vec<u8> = vec!();
        let res: &[u8] = &res1;
        assert_eq!(
            parse_byte_string(&[0x02, 0x0a, 0x50, 0x69, 0x6c, 0x6c, 0x65]),
            Ok((res, DataType::ByteString("Pille".to_string())))
        )
    }
    #[test]
    fn test_sbint() {
        let res1: Vec<u8> = vec!();
        let res: &[u8] = &res1;
        assert_eq!(parse_single_byte_int(&[0x06, 0x4C]),
                   Ok((res, DataType::SingleByteInteger(38)))
        )
    }
    #[test]
    fn test_fbint() {
        let res1: Vec<u8> = vec!();
        let res: &[u8] = &res1;
        assert_eq!(parse_four_bytes_int(&[0x07, 0x00, 0x00, 0x53, 0x32,]),
                   Ok((res, DataType::FourBytesInteger(10649)))
        )
    }
}
