/// enum with TCPLS errors
pub mod error;

use std::io::Write;
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

/// Handle creation and decoding of tcpls frame
pub struct Tcpls {
    max_size: usize,
    conn_id: usize,
    snd_buf: Vec<u8>,
    rcv_buf: Vec<u8>,
    highest_tls_seq: u64,
}

impl Tcpls {
    /// create a new tcpls object to handle tcpls frames
    pub fn new() -> Tcpls {
        Tcpls { 
            max_size: 16344, 
            conn_id:0, 
            snd_buf: vec![0; 16384], 
            rcv_buf: vec![0; 16384],
            highest_tls_seq: 0,
        }
    }

    /// gather all tcpls frames to create a record transmitted to tls
    pub fn create_record(&mut self, _payload: &[u8]) -> Vec<u8>{
        self.add_ping();
        self.snd_buf.clone()
    }

    /// read a tls record and parse every tcpls frame in it
    pub fn read_record(&mut self, payload: &[u8]) -> Result<(), Error> {
        // read buffer from the end for the 0-copy feature of tcpls
        for mut i in (0..payload.len()).rev() {
            match payload[i] {
                0x01 => {self.add_ack(); i -= 1;},
                0x04 => i -= self.read_ack(payload, i),
                _ => return Err(Error::UnknownTcplsType),
            }
        }
        Ok(())
    }

    /// read a ping frame and respond with a Ack
    fn read_ack(&self, payload: &[u8], mut index: usize) -> usize {
        index -= 1;
        let slice = &payload[index-4..index];
        let _conn_id = u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]);
        index -= 4;

        let slice = &payload[index-8..index];
        let _highest_tls_seq = u64::from_be_bytes([slice[0], slice[1], slice[2], slice[3],
                                                       slice[4], slice[5], slice[6], slice[7]]);
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
            self.snd_buf.push(0x01)
        }
 
    }

    fn add_ack(&mut self) {
        if self.snd_buf.len() + 13 < self.max_size {
            self.snd_buf.extend_from_slice(&self.highest_tls_seq.to_be_bytes());
            self.snd_buf.extend_from_slice(&self.conn_id.to_be_bytes());
            self.snd_buf.push(0x04);
        }
    }
}