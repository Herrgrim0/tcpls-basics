/// collection of functions to convert
/// a slice of bytes to a unsigned int of 16, 32 or 64 bits

pub(crate) mod conversion {
    use crate::tcpls::error::Error;

    pub(crate) fn slice_to_u16(bytes: &[u8]) -> Result<u16, Error> {
        if bytes.len() != 2 {
            Err(Error::BadSliceLength)
        } else {
            Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
        }
    }

    pub(crate) fn slice_to_u32(bytes: &[u8]) -> Result<u32, Error> {
        if bytes.len() != 4 {
            Err(Error::BadSliceLength)
        } else {
            Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
    }

    pub(crate) fn slice_to_u64(bytes: &[u8]) -> Result<u64, Error> {
        if bytes.len() != 8 {
            Err(Error::BadSliceLength)
        } else {
            Ok(u64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]))
        }
    }
}

/// All the useful constant for the TCPLS protocol
/// i.e: frame type, max record size
pub mod constant {
    use crate::msgs::fragmenter::MAX_FRAGMENT_LEN;

    /// padding frame
    pub const PADDING_FRAME: u8 = 0x00;
    /// ping frame
    pub const PING_FRAME: u8 = 0x01;
    pub(crate) const STREAM_FRAME: u8 = 0x02;
    pub(crate) const STREAM_FRAME_FIN: u8 = 0x03;
    pub(crate) const ACK_FRAME: u8 = 0x04;
    pub(crate) const NEW_TOKEN_FRAME: u8 = 0x05;
    pub(crate) const CONNECTION_RESET_FRAME: u8 = 0x06;
    pub(crate) const NEW_ADDRESS_FRAME: u8 = 0x07;
    pub(crate) const REMOVE_ADDRESS_FRAME: u8 = 0x08;
    pub(crate) const STREAM_CHANGE_FRAME: u8 = 0x09;

    pub(crate) const MAX_RECORD_SIZE: usize = MAX_FRAGMENT_LEN;
    pub(crate) const STREAM_HEADER_SIZE: usize = 15;
}
