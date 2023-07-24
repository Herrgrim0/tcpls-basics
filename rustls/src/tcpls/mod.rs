use std::collections::HashMap;

///handle tcpls errors
pub mod error;

///handle a tcpls stream
pub mod stream;

///collection of function to transform bytes to int
pub mod utils;

/// run tests
#[cfg(test)]
pub mod tcpls_test;

use log::trace;
use crate::tcpls::error::Error;
use crate::tcpls::stream::{TcplsStream, TcplsStreamBuilder};
use crate::tcpls::utils::{constant, conversion};

/// Differentiate the instanciation
/// of the TCPLS session
#[derive(Debug)]
pub enum Role {
    /// represent a client
    Client,
    /// represent a server
    Server,
}

/// Manage an underlying TCP/TLS connection
/// and all the Tcpls features above it
/// Work with the following event loop:
/// loop {
///     TCPLS_CONN process data (control & app) to send
///     TLS send data
///     TLS receive data
///     TCPLS_CONN process data (control & app) received
/// } -> concurrence can be implemented via mio
#[derive(Debug)]
pub struct TcplsConnection {
    // number refering the TCP connection
    conn_id: u32,

    // hashmap that link stream id to its stream objects
    streams: HashMap<u32,TcplsStream>,
    
    // remembering the last stream id given to avoir collision
    _last_stream_id_created: u32,

    // buffer to send data to the other party
    snd_buf: Vec<u8>, 

    //highest TLS record sequence for the ACK frame
    highest_tls_seq: u64,

    ack_received: bool,

    role: Role,

}


impl TcplsConnection {

    /// create new tcpls connection
    pub fn new(conn_id: u32, role: Role) -> Self {
        let stream1 = TcplsStreamBuilder::new(0);
        let mut streams = HashMap::new();
        streams.insert(0, stream1.build());
        Self { 
            conn_id, 
            streams, 
            _last_stream_id_created: 0, 
            snd_buf: Vec::new(), 
            highest_tls_seq:0, 
            ack_received: false,
            role,
        }
    }

    /// process the application and the control
    /// data to send
    pub fn process_w(&mut self) -> Option<Vec<u8>> {
        // application data
        let mut record: Vec<u8> = Vec::with_capacity(constant::MAX_RECORD_SIZE);

        for stream in self.streams.values_mut() {
            trace!("stream: {}, len: {}, offset {}", stream.get_id(), stream.get_len(), stream.get_offset());
            if record.len() >= constant::MAX_RECORD_SIZE {
                break;
            }

            if stream.has_data_to_send() {
                record.extend_from_slice(&stream.create_data_frame().unwrap_or_default());
            }
        }

        // control data
        if record.len() < constant::MAX_RECORD_SIZE {
            let space_left = constant::MAX_RECORD_SIZE - record.len();
            let mut i: usize = 0;
            while i < space_left {
                self.add_ping(&mut record);
                i += 1;
            }
        };

        match record.len() {
            0 => None,
            1 .. => Some(record),
            _ => panic!("record creation shitted itself"),
        }
    }

    // return the number of bytes read
    fn recv_stream(&mut self, payload: &[u8], mut offset: usize) -> usize {

        let stream_id: u32 = conversion::slice_to_u32(&payload[offset-4..offset]);
        offset-=4;
        if self.streams.contains_key(&stream_id) {
            let st = self.streams.get_mut(&stream_id).unwrap();
            st.read_record(&payload[..offset]) + 4
        } else {
            self.create_stream(payload, offset)
        }
    }

    /// create a new stream to process data
    fn create_stream(&mut self, payload: &[u8], mut offset: usize) -> usize {
        let new_stream_id: u32 = conversion::slice_to_u32(&payload[offset-4..offset]);
        offset -= 4;
        let mut n_stream = TcplsStream::new(new_stream_id, Vec::new());
        let consummed = n_stream.read_record(&payload[..offset]);
        self.streams.insert(new_stream_id, n_stream);

        consummed + 4
    }

    /// add a new stream to the current connection
    /// the stream has to be created beforehand with
    /// the TcplsStreamBuilder
    pub fn add_stream(&mut self, n_stream: TcplsStream, id: u32) {
        self.streams.insert(id, n_stream);
    }

    /// gather all tcpls frames to create a record transmitted to tls
    pub fn create_record(&mut self) -> Vec<u8>{
        let mut record: Vec<u8> =  match self.streams.get_mut(&0).unwrap().create_data_frame() {
            Some(frame) => frame,
            None => vec![],
        };

        record.extend_from_slice(&self.snd_buf);

        trace!("tcpls record with size {} is ready", record.len());
        self.snd_buf.clear();
        
        record
    }

    ///
    fn _create_control_frames(&self) -> Option<Vec<u8>> {
        todo!();
    }

    /// return connection ID
    pub fn get_id(&self) -> u32 {
        self.conn_id
    }

    /// probe each stream to see if there is still 
    /// data to send
    pub fn has_data(&self) -> bool {
        let mut ans: bool = false;
        for stream in self.streams.values() {
            ans = ans || stream.has_data_to_send();
        }
        trace!("{}", ans);
        ans
    }

    /// read a tls record and parse every tcpls frame in it
    pub fn process_r(&mut self, payload: &Vec<u8>) -> Result<(), Error> {
        // read buffer from the end for the 0-copy feature of tcpls

        let mut i = payload.len() - 1;
 
        while i > 0 {
            let consummed = self.process_frame(payload, i)?;
            if i > consummed {
                i -= consummed;
            } else {
                i = 0;
            }
            
        }

        // safeguard because main loop doesn't handle the
        // first byte of the payload
        if payload[0] == constant::PING_FRAME {
            trace!("Ping Frame received");
                self.add_ack();
        }
        Ok(())
    }

    // match the payload with a possible frame and process it
    fn process_frame(&mut self, payload: &Vec<u8>, i: usize) -> Result<usize, Error> {
        let mut consummed = 0;
        match payload[i] {
            constant::PADDING_FRAME => {
                trace!("Padding Frame received"); 
                consummed += 1
            },
            constant::PING_FRAME => {
                trace!("Ping Frame received");
                self.add_ack(); 
                consummed = 1; 
            },
            constant::ACK_FRAME => {
                //trace!("Ack Frame received");
                self.ack_received = true;
                consummed = self.read_ack(payload, i)
            },
            constant::STREAM_FRAME | constant::STREAM_FRAME_FIN => {
                trace!("Stream frame received");
                consummed += self.recv_stream(payload, i);
                },
            constant::NEW_TOKEN_FRAME => todo!(),
            constant::CONNECTION_RESET_FRAME => todo!(),
            constant::NEW_ADDRESS_FRAME => todo!(),
            constant::REMOVE_ADDRESS_FRAME => todo!(),
            constant::STREAM_CHANGE_FRAME => todo!(),
            _ => {
                trace!("Unknown tcpls type {}, index: {}", payload[i], i);
                return Err(Error::UnknownTcplsType)
            },
        }

        Ok(consummed)
    }

    /// read a ping frame and respond with a Ack
    fn read_ack(&self, payload: &Vec<u8>, mut offset: usize) -> usize {
        assert_eq!(payload[offset], constant::ACK_FRAME);
        offset -= 1; // remove frame value
        
        let conn_id = conversion::slice_to_u32(&payload[offset-4..offset]);
        offset -= 4;

        let highest_tls_seq: u64 = if offset < 8 {
            // to avoid attempt to subtract with overflow
            // because of index problem
            conversion::slice_to_u64(&payload[0..offset+1])
        } else {
            conversion::slice_to_u64(&payload[offset-8..offset])
        };

        trace!("Ack frame received on conn: {}, highest tls seq: {}", conn_id, highest_tls_seq);

        13 // len of an ACK frame
    }

    /// update the highest tls seq, mainly for the ack
    pub fn update_tls_seq(&mut self, tls_seq: u64) {
        self.highest_tls_seq = tls_seq;
    }

    /// fill a stream w/ data to send
    pub fn get_data(&mut self, data: &[u8]) {
        //self.streams.get(&0).unwrap().get_data(data);
        self.streams.get_mut(&0).unwrap().get_data(data);
    }

    fn add_ping(&mut self, record: &mut Vec<u8>) {
        if record.len() < constant::MAX_RECORD_SIZE {
            record.push(constant::PING_FRAME);
        }
    }

    fn add_ack(&mut self) {
        if self.snd_buf.len() + 13 < constant::MAX_RECORD_SIZE {
            self.snd_buf.extend_from_slice(&self.highest_tls_seq.to_be_bytes());
            self.snd_buf.extend_from_slice(&self.conn_id.to_be_bytes());
            self.snd_buf.push(constant::ACK_FRAME);
        }
    }

    /// return data processed by a stream
    pub fn get_stream_data(&self, id: u32) -> Result<&[u8], Error> {
        if self.streams.contains_key(&id) {
            Ok(self.streams.get(&id).unwrap().get_stream_data())
        } else {
            Err(Error::UnknownTcplsType)
        }
    }

    /// display streams hashmap for debug purpose
    pub fn dbg_streams(&self) {
        for (k, v) in &self.streams {
            trace!("{}, {}", k, v.get_len());
        }
    }

    /// inverse ack_received member
    pub fn inv_ack(&mut self) {
        self.ack_received = !self.ack_received;
    }

    /// return the state of ack_received member
    pub fn has_received_ack(&self) -> bool {
        self.ack_received
    }

    ///give the role of the TcplsConnection instance
    pub fn get_role(self) -> Role {
        self.role
    }
}
