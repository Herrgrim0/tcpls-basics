
use log::trace;

/// Management of a TCPLS stream
use crate::tcpls::convert;

// Max size of a TLS record minus size of
// a TCPLS headers for stream data frame (16 bytes)
const MAX_DATA_SIZE: usize = usize::pow(2, 14) - 3325; // error when using write_tls with size above this value

/// Manage a tcpls stream
#[derive(Debug)]
pub struct TcplsStream {
    stream_id: u32,
    offset: u64,
    snd_data : Vec<u8>,
    rcv_data: Vec<u8>,
}

impl TcplsStream {
    /// create a new stream
    pub fn new(stream_id: u32, r_data: Vec<u8>) -> TcplsStream {
        TcplsStream {stream_id,
                     offset: 0,
                     snd_data: Vec::new(),
                     rcv_data: r_data,
        }
    }

    /// receive a vector where frame type and 
    /// stream id bytes have been removed
    /// return the number of bytes read
    pub fn read_record(&mut self, new_data: &[u8]) -> usize {
        let mut cursor: usize = new_data.len();

        let stream_offset: u64 = convert::slice_to_u64(&new_data[cursor-8..cursor]);
        cursor -=8;

        self.offset += stream_offset;

        let stream_len: u16 = convert::slice_to_u16(&new_data[cursor-2..cursor]);
        cursor -= 2;
        trace!("cursor: {}, stream_len: {}", cursor, stream_len);
        self.rcv_data.extend_from_slice(&new_data[cursor-stream_len as usize..cursor]);

        stream_len as usize + 10
    }

    /// return a vec that fits in a TLS record
    pub fn create_data_frame(&mut self) -> Option<Vec<u8>> {
        let mut frame: Vec<u8> = Vec::new(); // TODO: decide if still local var or struct mmbr 
        let mut cp_len: u16 = MAX_DATA_SIZE as u16;
        let mut typ: u8 = 0x02;

        if self.snd_data.is_empty() {
            return None;
        };

        if self.snd_data[self.offset as usize..].len() >= MAX_DATA_SIZE {
            frame.extend_from_slice(&self.snd_data[self.offset as usize..self.offset as usize+MAX_DATA_SIZE]);
        } else {
            frame.extend_from_slice(&self.snd_data[self.offset as usize..]);
            cp_len = (self.snd_data.len() - self.offset as usize) as u16;
            typ = 0x03;
        }

        self.offset += cp_len as u64;

        self.add_meta_data_to_frame(&mut frame, cp_len, typ);
        trace!("stream {} created data frame", self.stream_id);
        
        Some(frame)
    }

    fn add_meta_data_to_frame(&self, frame: &mut Vec<u8>, len: u16, type_value: u8) {
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(&self.offset.to_be_bytes());
        frame.extend_from_slice(&self.stream_id.to_be_bytes());
        frame.push(type_value);
    }

    /// retrieve data to send
    pub fn get_data(&mut self, data: &[u8]) -> usize {
        self.snd_data.extend_from_slice(&data);

        self.snd_data.len()
    }

    /// return a ref to the app data received
    pub fn get_stream_data(&self) -> &[u8] {
        &self.rcv_data
    }

    /// return true if there is still data to
    /// send. Compare offset and len of the buffer
    /// of data to send to do so.
    pub fn has_data_to_send(&self) -> bool {
        return self.snd_data.len() > self.offset as usize
    }

    /// return id
    pub fn get_id(&self) -> u32 {
        self.stream_id
    }

    /// return len data to send
    pub fn get_len(&self) -> usize {
        self.snd_data.len()
    }

    /// return current offset
    pub fn get_offset(&self) -> u64 {
        self.offset
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
    pub fn new(stream_id: u32) -> TcplsStreamBuilder{
        TcplsStreamBuilder { stream_id, snd_data: Vec::new() }
    }

    /// add data to the stream
    pub fn add_data(&mut self, data: &[u8]) {
        self.snd_data.extend_from_slice(data);
    }

    /// consumme the builder to create a stream
    pub fn build(self) -> TcplsStream {
        TcplsStream { stream_id: self.stream_id, offset: 0, snd_data: self.snd_data, rcv_data: Vec::new() }
    }
}