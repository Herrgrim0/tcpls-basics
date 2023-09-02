use log::trace;

/// Management of a TCPLS stream
use crate::tcpls::utils::*;
const OFFSET_SIZE: usize = 8;
const LENGTH_SIZE: usize = 2;

// Max size of a TLS record minus size of
// a TCPLS headers for stream data frame (16 bytes)

/// Manage a tcpls stream
#[derive(Debug)]
pub struct TcplsStream {
    stream_id: u32,
    rcv_offset: u64,
    snd_offset: u64,
    snd_data: Vec<u8>,
    rcv_data: Vec<u8>,
}

impl TcplsStream {
    /// create a new stream
    pub fn new(stream_id: u32, r_data: Vec<u8>) -> Self {
        Self {
            stream_id,
            rcv_offset: 0,
            snd_offset: 0,
            snd_data: Vec::new(),
            rcv_data: r_data,
        }
    }

    /// receive a vector where frame type and
    /// stream id bytes have been removed
    /// return the number of bytes read
    pub fn read_stream_frame(&mut self, new_data: &[u8]) -> usize {
        trace!("stream {} has a data frame", self.stream_id);
        let mut cursor: usize = new_data.len();

        let stream_offset: u64 = conversion::slice_to_u64(&new_data[cursor - OFFSET_SIZE..cursor])
            .expect("Failed to convert bytes");
        cursor -= OFFSET_SIZE;

        self.rcv_offset = stream_offset;

        let stream_len: u16 = conversion::slice_to_u16(&new_data[cursor - LENGTH_SIZE..cursor])
            .expect("Failed to convert bytes");
        cursor -= LENGTH_SIZE;
        trace!(
            "{} - cursor: {}, stream_len: {}, offset: {}",
            self.stream_id,
            cursor,
            stream_len,
            self.rcv_offset
        );

        self.rcv_data
            .extend_from_slice(&new_data[cursor - stream_len as usize..cursor]);

        stream_len as usize + OFFSET_SIZE + LENGTH_SIZE
    }

    /// return a vec that fits in a TLS record
    /// max_size is the maximal length the data frame
    /// must be to fit in a record
    pub fn create_stream_frame(&mut self, max_size: usize) -> Option<Vec<u8>> {
        let mut frame: Vec<u8> = Vec::new();
        let mut typ: u8 = constant::STREAM_FRAME;

        if self.snd_data.is_empty() {
            return None;
        };

        if self.snd_data[self.snd_offset as usize..].len() + constant::STREAM_HEADER_SIZE > max_size
        {
            let data_size = self.snd_offset as usize + max_size - constant::STREAM_HEADER_SIZE;
            frame.extend_from_slice(&self.snd_data[self.snd_offset as usize..data_size]);
        } else {
            frame.extend_from_slice(&self.snd_data[self.snd_offset as usize..]);
            typ = constant::STREAM_FRAME_FIN;
        }

        let cp_len = frame.len() as u16;
        self.add_meta_data_to_frame(&mut frame, cp_len, typ);
        self.snd_offset += cp_len as u64;

        trace!(
            "stream {} created data frame of len {}",
            self.stream_id,
            cp_len
        );

        Some(frame)
    }

    fn add_meta_data_to_frame(&self, frame: &mut Vec<u8>, len: u16, type_value: u8) {
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(&self.snd_offset.to_be_bytes());
        frame.extend_from_slice(&self.stream_id.to_be_bytes());
        frame.push(type_value);
        trace!(
            "{:?}",
            &frame[frame.len() - constant::STREAM_HEADER_SIZE..frame.len()]
        );
    }

    /// return a ref to the app data received
    pub fn get_stream_data(&self) -> &[u8] {
        &self.rcv_data
    }

    /// add a slice of data to the sending buffer of the
    /// stream
    pub fn add_data_to_send(&mut self, data: &[u8]) {
        self.snd_data.extend_from_slice(data);
    }

    /// return true if there is still data to
    /// send. Compare offset and len of the buffer
    /// of data to send to do so.
    pub fn has_data_to_send(&self) -> bool {
        !self.snd_data.is_empty() && (self.snd_data.len() > self.snd_offset as usize)
    }

    /// return id
    pub fn get_id(&self) -> u32 {
        self.stream_id
    }

    /// return length of the buffer
    /// that send data
    pub fn get_len_snd_buf(&self) -> usize {
        self.snd_data.len()
    }

    /// return length of the buffer
    /// that received data
    pub fn get_len_recv_buf(&self) -> usize {
        self.rcv_data.len()
    }

    /// return current offset
    pub fn get_snd_offset(&self) -> u64 {
        self.snd_offset
    }

    /// return current offset of data received
    pub fn get_rcv_offset(&self) -> u64 {
        self.rcv_offset
    }
}

/// Struct to build stream with different configuration
/// e.g. instantiate a stream with data to send
pub struct TcplsStreamBuilder {
    stream_id: u32,
    snd_data: Vec<u8>,
}

impl TcplsStreamBuilder {
    /// create a new builder of stream
    pub fn new(stream_id: u32) -> Self {
        Self {
            stream_id,
            snd_data: Vec::new(),
        }
    }

    /// add data to the stream
    pub fn add_data(&mut self, data: &[u8]) {
        self.snd_data.extend_from_slice(data);
    }

    /// consumme the builder to create a stream
    pub fn build(self) -> TcplsStream {
        TcplsStream {
            stream_id: self.stream_id,
            rcv_offset: 0,
            snd_offset: 0,
            snd_data: self.snd_data,
            rcv_data: Vec::new(),
        }
    }
}
