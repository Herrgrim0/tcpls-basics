/// enum with TCPLS errors
pub mod error;

use std::io::Write;
use log::trace;

use crate::tcpls::error::Error;

/*enum_builder! {
    /// Tcpls frame type
    @U8
    EnumName: TcplsFrameType;
    EnumVal{
        Padding => 0x00,
        Ping => 0x01,
        Ack => 0x04
    }
}*/

// minimum length of a tcpls stream frame containing a chunk of data of 1 byte
const MIN_STREAM_LEN: usize = 16; 

const PADDING_FRAME: u8 = 0x00;
const PING_FRAME: u8 = 0x01;
const STREAM_FRAME: u8 = 0x02;
const STREAM_FRAME_FIN: u8 = 0x03;
const ACK_FRAME: u8 = 0x04;
const NEW_TOKEN_FRAME: u8 = 0x05;
const CONNECTION_RESET_FRAME: u8 = 0x06;
const NEW_ADDRESS_FRAME: u8 = 0x07;
const REMOVE_ADDRESS_FRAME: u8 = 0x08;
const STREAM_CHANGE_FRAME: u8 = 0x09;

/// Handle one tcpls connection
/// i.e 1 tcp stream
//struct TcplsConnection {}


/// Handle creation and decoding of tcpls frame
pub struct Tcpls {
    max_size: usize,
    conn_id: usize,
    stream_id: usize,
    snd_buf: Vec<u8>,
    rcv_buf: Vec<u8>,
    highest_tls_seq: u64,
}

impl Tcpls {
    /// create a new tcpls object to handle tcpls frames
    pub fn new() -> Tcpls {
        Tcpls { 
            max_size: 16344, 
            conn_id: 0,
            stream_id: 0,
            snd_buf: vec![], 
            rcv_buf: vec![],
            highest_tls_seq: 0,
        }
    }

    /// gather all tcpls frames to create a record transmitted to tls
    pub fn create_record(&mut self, payload: &[u8]) -> Vec<u8>{
        trace!("creating tcpls record");
        self.snd_buf.extend_from_slice(payload);
        self.snd_buf.push(STREAM_FRAME);
        self.add_ping();
        trace!("tcpls record: {:?}", self.snd_buf);
        return self.snd_buf.clone()
    }

    /// read a tls record and parse every tcpls frame in it
    pub fn read_record(&mut self, payload: &Vec<u8>) -> Result<(), Error> {
        // read buffer from the end for the 0-copy feature of tcpls
        let mut i = payload.len() - 1;
        let mut consummed: usize = 0;
        while i > 0 {
            match payload[i] {
                PADDING_FRAME => trace!("Padding Frame received"),
                PING_FRAME => {
                    trace!("Ping Frame received");
                    self.add_ack(); 
                    consummed = 1; },
                ACK_FRAME => {
                    consummed = self.read_ack(payload, i)},
                STREAM_FRAME => {
                    consummed = self.read_stream(payload, i)},
                NEW_TOKEN_FRAME => todo!(),
                CONNECTION_RESET_FRAME => todo!(),
                NEW_ADDRESS_FRAME => todo!(),
                REMOVE_ADDRESS_FRAME => todo!(),
                STREAM_CHANGE_FRAME => todo!(),
                _ => {
                    trace!("Unknown tcpls type {}, index: {}", payload[i], i);
                    return Err(Error::UnknownTcplsType)
                },
            }
            i = i - consummed;
            trace!("index is: {}", i);
        }
        Ok(())
    }

    /// read a ping frame and respond with a Ack
    fn read_ack(&self, payload: &Vec<u8>, mut offset: usize) -> usize {
        offset -= 1;
        
        let conn_id = Tcpls::slice_to_u32(&payload[offset-4..offset]);
        offset -= 4;

        let highest_tls_seq = Tcpls::slice_to_u64(&payload[offset-8..offset]);

        trace!("Ack frame received on conn: {}, highest tls seq: {}", conn_id, highest_tls_seq);

        13
    }

    /// empty the vector containing a record
    pub fn flush(&mut self) {
        let _ = self.snd_buf.flush();
    }

    /// update the highest tls seq, mainly for the ack
    pub fn update_tls_seq(&mut self, tls_seq: u64) {
        self.highest_tls_seq = tls_seq;
    }

    fn add_ping(&mut self) {
        if self.snd_buf.len() < self.max_size {
            self.snd_buf.push(PING_FRAME);
        }
 
    }

    fn add_ack(&mut self) {
        if self.snd_buf.len() + 13 < self.max_size {
            self.snd_buf.extend_from_slice(&self.highest_tls_seq.to_be_bytes());
            self.snd_buf.extend_from_slice(&self.conn_id.to_be_bytes());
            self.snd_buf.push(ACK_FRAME);
        }
    }

    fn _add_stream(&mut self) {
        if self.snd_buf.len() < self.max_size - MIN_STREAM_LEN {

        }
    }

    fn read_stream(&mut self, payload: &Vec<u8>, mut offset: usize) -> usize {
        trace!("reading a stream: {}", payload[offset]);
        offset -= 1; // avoid type byte

        let stream_id: u32 = Tcpls::slice_to_u32(&payload[offset-4..offset]);
        offset-=4;

        let stream_offset: u64 = Tcpls::slice_to_u64(&payload[offset-8..offset]);
        offset -=8;

        let stream_len: u16 = Tcpls::slice_to_u16(&payload[offset-2..offset]);
        offset -= 2;

        self.rcv_buf.extend_from_slice(&payload[offset-stream_len as usize..offset]);

        self.rcv_buf.len() + 1
    }

    /// display data received in a tcpls record
    pub fn display_rcv_data(&self) {
        let s = match std::str::from_utf8(&self.rcv_buf) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        println!("{}", s);
    }

    fn slice_to_u16(bytes: &[u8]) -> u16 {
        assert_eq!(bytes.len(), 2);
        u16::from_be_bytes([bytes[0], bytes[1]])
    }

    fn slice_to_u32(bytes: &[u8]) -> u32 {
        assert_eq!(bytes.len(), 4);
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn slice_to_u64(bytes: &[u8]) -> u64 {
        assert_eq!(bytes.len(), 8);
        u64::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3],
                            bytes[4], bytes[5], bytes[6], bytes[7]])
    }
}