/// Management of a TCPLS stream
use crate::tcpls::convert;

// Max size of a TLS record minus size of
// a TCPLS headers for stream data frame (16 bytes)
const MAX_DATA_SIZE: usize = 16368;

pub struct TcplsStream {
    stream_id: u32,
    offset: u64,
    snd_data : Vec<u8>,
    rcv_data: Vec<u8>,
    frame: Vec<u8>,
}

impl TcplsStream {
    pub fn new(stream_id: u32, r_data: Vec<u8>) -> TcplsStream {
        TcplsStream {stream_id,
                     offset: 0,
                     snd_data: Vec::new(),
                     rcv_data: r_data,
                     frame: Vec::with_capacity(MAX_DATA_SIZE + 16),
                    }
    }

    /// receive a vector where frame type and 
    /// stream id bytes have been removed
    pub fn read_record(&self, new_data: &Vec<u8>) {
        let mut cursor: usize = new_data.len();
        let stream_offset: u64 = convert::slice_to_u64(&new_data[cursor-8..cursor]);
        cursor -=8;

        self.offset += stream_offset;

        let stream_len: u16 = convert::slice_to_u16(&new_data[cursor-2..cursor]);
        cursor -= 2;

        self.rcv_data.extend_from_slice(&new_data[cursor-stream_len as usize..cursor]);
    }

    /// return a vec that fits in a TLS record
    pub fn create_stream_data_frame(&self) -> Vec<u8> {
        let mut frame: Vec<u8> = Vec::new(); // TODO: decide if still local var or struct mmbr 

        if self.snd_data[self.offset as usize..].len() >= frame.len() {
            frame.copy_from_slice(&self.snd_data[self.offset as usize..self.offset as usize+MAX_DATA_SIZE]);
            let cp_len = MAX_DATA_SIZE as u16;
            self.add_meta_data_to_frame(&mut frame, cp_len, 0x02);
        } else {
            frame.copy_from_slice(&self.snd_data[self.offset as usize..]);
            let cp_len = (self.snd_data.len() - self.offset as usize) as u16;
            self.add_meta_data_to_frame(&mut frame, cp_len, 0x03);
        }
        
        frame
    }

    fn add_meta_data_to_frame(&self, frame: &mut Vec<u8>, len: u16, type_value: u8) {
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(&self.offset.to_be_bytes());
        frame.extend_from_slice(&self.stream_id.to_be_bytes());
        frame.push(type_value);
    }
}

/// Struct to build stream with different configuration
/// e.g. instantiate a stream with data to send
pub struct TcplsStreamBuilder {
    stream_id: u32,
    rcv_data: Vec<u8>,
    snd_data: Vec<u8>,
}

impl TcplsStreamBuilder {
    pub fn new(stream_id: u32) -> TcplsStreamBuilder{
        TcplsStreamBuilder { stream_id, rcv_data: Vec::new(), snd_data: Vec::new() }
    }

    pub fn data_to_read(&mut self, data: &[u8]) {
        self.snd_data = data.to_vec();
    }

    pub fn build(&self) -> TcplsStream {
        TcplsStream { stream_id: self.stream_id, offset: 0, snd_data: self.snd_data, rcv_data: self.rcv_data, frame: Vec::new() }
    }
}