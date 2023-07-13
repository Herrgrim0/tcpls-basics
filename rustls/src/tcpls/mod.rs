use std::io::Write;

///handle tcpls errors
pub mod error;

///handle a tcpls stream
pub mod stream;

///collection of function to transform bytes to int
pub mod convert;

use log::trace;
use std::collections::HashMap;

use crate::tcpls::error::Error;
use crate::tcpls::stream::TcplsStream;

use self::stream::TcplsStreamBuilder;

// minimum length of a tcpls stream frame containing a chunk of data of 1 byte
const MAX_RECORD_SIZE: usize = 16384;
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

/// Handle creation and decoding of tcpls frame
pub struct Tcpls {
    conn_id: u32,
    streams: HashMap<u32, TcplsStream>,
    last_stream_id_created: u32,
    snd_buf: Vec<u8>, // buffer to send data to the other party
    rcv_buf: Vec<u8>, // buffer to receive data from the other party
    highest_tls_seq: u64,
}

impl Tcpls {
    /// create a new tcpls object to handle tcpls frames
    pub fn new() -> Tcpls {
        let mut stream_1 = TcplsStreamBuilder::new(0).build();
        let mut streams: HashMap<u32, TcplsStream> = HashMap::new();
        streams.insert(0, stream_1);

        Tcpls { 
            conn_id: 0,
            streams,
            last_stream_id_created: 0,
            snd_buf: Vec::with_capacity(MAX_RECORD_SIZE), 
            rcv_buf: Vec::with_capacity(MAX_RECORD_SIZE),
            highest_tls_seq: 0,
        }
    }
}


/// Manage an underlying TCP/TLS connection
/// and all the Tcpls features above it

pub struct TcplsConnection {
    // number refering the TCP connection
    conn_id: u32,

    // hashmap that link stream id to its stream objects
    streams: TcplsStream,
    
    // remembering the last stream id given to avoir collision
    last_stream_id_created: u32,

    // buffer to send data to the other party
    snd_buf: Vec<u8>, 
    
    // buffer to receive data from the other party
    rcv_buf: Vec<u8>,

    //highest TLS record sequence for the ACK frame
    highest_tls_seq: u64,
}


impl TcplsConnection {
    fn new(conn_id: u32) -> TcplsConnection {
        let mut stream1 = TcplsStreamBuilder::new(0);
        TcplsConnection { conn_id, 
            streams: stream1.build(), 
            last_stream_id_created: 0, 
            snd_buf: Vec::new(), 
            rcv_buf: Vec::with_capacity(16384), 
            highest_tls_seq:0 ,
            //tls_conn: 
        }
    }

    fn send(&mut self) {
        self.snd_buf = self.streams.create_stream_data_frame();
    }

    fn recv_stream(&mut self, mut offset: usize) {
        let mut cursor: usize = 0;

        let stream_id: u32 = convert::slice_to_u32(&self.rcv_buf[offset-4..offset]);
        offset-=3;
        
        self.streams.read_record(&self.rcv_buf[..offset]);
    
    }

    fn create_stream(&self) {
        let new_stream_id = self.last_stream_id_created + 2;
        let stream = TcplsStream::new(new_stream_id, self.snd_buf.clone());
    }

    /// gather all tcpls frames to create a record transmitted to tls
    pub fn create_record(&mut self, payload: &[u8]) -> Vec<u8>{
        trace!("creating tcpls record");
        //self.add_stream(payload);
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
                STREAM_FRAME | STREAM_FRAME_FIN => {
                    self.recv_stream(i);
                    i = 0; },
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
        trace!("Ack frame received");
        offset -= 1;
        
        let conn_id = convert::slice_to_u32(&payload[offset-4..offset]);
        offset -= 4;

        let highest_tls_seq = convert::slice_to_u64(&payload[offset-8..offset]);

        trace!("Ack frame received on conn: {}, highest tls seq: {}", conn_id, highest_tls_seq);

        13 // len of an ACK frame
    }

    /// update the highest tls seq, mainly for the ack
    pub fn update_tls_seq(&mut self, tls_seq: u64) {
        self.highest_tls_seq = tls_seq;
    }

    fn add_ping(&mut self) {
        if self.snd_buf.len() < MAX_RECORD_SIZE {
            self.snd_buf.push(PING_FRAME);
        }
    }

    fn add_ack(&mut self) {
        if self.snd_buf.len() + 13 < MAX_RECORD_SIZE {
            self.snd_buf.extend_from_slice(&self.highest_tls_seq.to_be_bytes());
            self.snd_buf.extend_from_slice(&self.conn_id.to_be_bytes());
            self.snd_buf.push(ACK_FRAME);
        }
    }
}