/// collection of functions to convert
/// a slice of bytes to a unsigned int of 16, 32 or 64 bits


pub(crate) fn slice_to_u16(bytes: &[u8]) -> u16 {
    assert_eq!(bytes.len(), 2);
    u16::from_be_bytes([bytes[0], bytes[1]])
}

pub(crate) fn slice_to_u32(bytes: &[u8]) -> u32 {
    assert_eq!(bytes.len(), 4);
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(crate) fn slice_to_u64(bytes: &[u8]) -> u64 {
    assert_eq!(bytes.len(), 8);
    u64::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3],
                        bytes[4], bytes[5], bytes[6], bytes[7]])
}