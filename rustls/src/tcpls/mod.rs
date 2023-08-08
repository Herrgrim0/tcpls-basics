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
    last_stream_id_created: u32,

    // buffer to keep control data before
    // copy in a record
    ctrl_buf: Vec<u8>, 

    //highest TLS record sequence for the ACK frame
    internal_highest_record_sequence: u64,

    ack_received: bool,

    role: Role,

    // for demo purpose, keep the last stream updated
    last_stream_processed: u32,

    // keep track of the last ack received for demo purpose
    highest_record_sequence_received: u64,

}


impl TcplsConnection {

    /// create new tcpls connection with an empty stream
    pub fn new(conn_id: u32, role: Role) -> Self {
        let stream1 = TcplsStreamBuilder::new(0);
        let mut streams = HashMap::new();
        streams.insert(0, stream1.build());
        Self { 
            conn_id, 
            streams, 
            last_stream_id_created: 0, 
            ctrl_buf: Vec::new(), 
            internal_highest_record_sequence:0, 
            ack_received: false,
            role,
            last_stream_processed: 0,
            highest_record_sequence_received: 0,
        }
    }

    /// create record, a list of one or more TCPLS frame(s)
    pub fn create_record(&mut self) -> Result<Vec<u8>, Error> {
        // application data
        let mut record: Vec<u8> = Vec::with_capacity(constant::MAX_RECORD_SIZE);
        let mut space_left = constant::MAX_RECORD_SIZE - self.ctrl_buf.len();

        for stream in self.streams.values_mut() {
            trace!("stream: {}, len: {}, offset {}", stream.get_id(), stream.get_len_snd_buf(), stream.get_offset());

            if stream.has_data_to_send() && space_left > constant::MIN_STREAM_DATA_SIZE  {
                record.extend_from_slice(&stream.create_stream_frame(space_left).unwrap_or_default());
                space_left = constant::MAX_RECORD_SIZE - record.len();
            }
        }

        // control data at the end of the record
        if record.len() < constant::MAX_RECORD_SIZE - self.ctrl_buf.len() {
            record.extend_from_slice(&self.ctrl_buf);
            self.ctrl_buf.clear();
        };

        match record.len() {
            0 => Ok(vec![0]),
            1 ..=constant::MAX_RECORD_SIZE => Ok(record),
            _ => Err(Error::UnexpectedRecordSize),
        }
    }

    /// read a tls record from end to start, parse every tcpls frame in it
    pub fn process_record(&mut self, payload: &Vec<u8>) -> Result<(), Error> {
        // read buffer from the end for the 0-copy feature of tcpls

        let mut i = payload.len() - 1;
 
        while i > 0 {
            let consumed = self.process_frame(payload, i).expect("error while reading record");
            if i > consumed {
                i -= consumed;
            } else {
                i = 0;
            }
            
        }

        // safeguard because main loop doesn't handle the
        // first byte of the payload
        if payload[0] == constant::PING_FRAME {
            trace!("Ping Frame received");
                self.add_ack_frame();
        }
        Ok(())
    }

    // Match the tcpls frame with the last byte of the payload.
    // This byte represent the value of a frame type.
    fn process_frame(&mut self, payload: &Vec<u8>, i: usize) -> Result<usize, Error> {
        let mut consumed = 0;
        match payload[i] {
            constant::PADDING_FRAME => {
                trace!("Padding Frame received"); 
                consumed += 1;
            },
            constant::PING_FRAME => {
                trace!("Ping Frame received");
                self.add_ack_frame(); 
                consumed += 1; 
            },
            constant::ACK_FRAME => {
                //trace!("Ack Frame received");
                self.ack_received = true;
                consumed = self.read_ack(payload, i);
            },
            constant::STREAM_FRAME | 
            constant::STREAM_FRAME_FIN => {
                trace!("Stream frame received");
                consumed += self.recv_stream_frame(payload, i) + 1 // the type frame;
                },
            constant::NEW_TOKEN_FRAME => unreachable!(),
            constant::CONNECTION_RESET_FRAME => unreachable!(),
            constant::NEW_ADDRESS_FRAME => unreachable!(),
            constant::REMOVE_ADDRESS_FRAME => unreachable!(),
            constant::STREAM_CHANGE_FRAME => unreachable!(),
            _ => {
                trace!("Unknown tcpls type {}, index: {}", payload[i], i);
                return Err(Error::UnknownTcplsType)
            },
        }

        Ok(consumed)
    }
    // return the number of bytes read
    fn recv_stream_frame(&mut self, payload: &[u8], mut offset: usize) -> usize {

        let stream_id: u32 = conversion::slice_to_u32(&payload[offset-4..offset])
                                .expect("Failed to convert bytes");
        offset -= 4;
        let st = self.streams.entry(stream_id)
            .or_insert(TcplsStream::new(stream_id, Vec::new()));

        self.last_stream_processed = stream_id;
        st.read_stream_frame(&payload[..offset]) + 4 // bytes removed from offset
    }

    /// add a new stream to the current connection
    /// the stream has to be created beforehand with
    /// the TcplsStreamBuilder
    pub fn attach_stream(&mut self, n_stream: TcplsStream, id: u32) {
        self.streams.insert(id, n_stream);
        self.last_stream_id_created = id;
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

    /// read a ping frame and respond with a Ack
    fn read_ack(&mut self, payload: &Vec<u8>, mut offset: usize) -> usize {
        //offset -= 1; // remove frame value
        
        let conn_id = conversion::slice_to_u32(&payload[offset-4..offset])
                                .expect("Failed to convert bytes");
        offset -= 4;

        let highest_tls_seq: u64 = if offset < 8 {
            // to avoid attempt to subtract with overflow
            // because of index problem
            conversion::slice_to_u64(&payload[0..offset+1]).expect("Failed to convert bytes")
        } else {
            conversion::slice_to_u64(&payload[offset-8..offset]).expect("Failed to convert bytes")
        };

        trace!("Ack frame received on conn: {}, highest tls seq: {}", conn_id, highest_tls_seq);

        self.highest_record_sequence_received = highest_tls_seq;
        13 // len of an ACK frame
    }

    /// update the highest tls seq, mainly for the ack
    pub fn update_tls_seq(&mut self, tls_seq: u64) {
        self.internal_highest_record_sequence = tls_seq;
    }

    /// return the highest record sequence of the
    /// underlying tls connection
    pub fn get_highest_tls_record_seq(&self) -> u64 {
        self.internal_highest_record_sequence
    }

    /// fill a stream w/ data to send
    pub fn set_data(&mut self, data: &[u8]) {
        //self.streams.get(&0).unwrap().get_data(data);
        self.streams.get_mut(&0).unwrap().get_data(data);
    }

    fn _add_ping_frame(&mut self, record: &mut Vec<u8>) {
        if record.len() < constant::MAX_RECORD_SIZE {
            record.push(constant::PING_FRAME);
        }
    }

    /// add an ACK frame in the sending buffer.
    pub fn add_ack_frame(&mut self) {
        if self.ctrl_buf.len() + 13 < constant::MAX_RECORD_SIZE {
            self.ctrl_buf.extend_from_slice(&self.internal_highest_record_sequence.to_be_bytes());
            self.ctrl_buf.extend_from_slice(&self.conn_id.to_be_bytes());
            self.ctrl_buf.push(constant::ACK_FRAME);
        }
    }

    /// return data processed by a stream
    pub fn get_stream_data(&self, id: u32) -> Result<&[u8], Error> {
        if self.streams.contains_key(&id) {
            Ok(self.streams.get(&id).unwrap().get_stream_data())
        } else {
            Err(Error::StreamNotFound)
        }
    }

    /// add data to send to the last stream created
    pub fn set_stream_data(&mut self, data: &[u8]) {
        self.streams.get_mut(&self.last_stream_id_created)
                    .unwrap()
                    .add_data_to_send(data);
    }

    /// display streams hashmap for debug purpose
    pub fn dbg_streams(&self) {
        for (k, v) in &self.streams {
            trace!("{}, {}", k, v.get_len_snd_buf());
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

    /// return a string with info about last stream processed
    pub fn get_last_stream_processed_info(&self) -> String {
        let stream = self.streams.get(&self.last_stream_processed).expect("Failed to get Stream");
        format!("Stream ID: {}
                  offset: {}
                  current length: {}", 
                stream.get_id(), stream.get_offset(), stream.get_len_recv_buf())
    }

    /// return highest record sequence received in a string
    pub fn get_last_ack_info(&self) -> String {
        format!("Highest record sequence received: {}", self.highest_record_sequence_received)
    }

    /// return a string with the info of each stream,
    /// its id and the data it received.
    pub fn get_streams_received_info(&self) -> String {
        let mut info = String::new();
        for stream in self.streams.values() {
            info.push_str(&format!("\nStream {} received {} bytes.\n", 
                                          stream.get_id(),
                                          stream.get_len_recv_buf()))
        }
        info
    }

    /// return a string with the info of each stream,
    /// its id and the data it sent.
    pub fn get_streams_sent_info(&self) -> String {
        let mut info = String::new();
        for stream in self.streams.values() {
            info.push_str(&format!("\nStream {} received {} bytes.\n", 
                                          stream.get_id(),
                                          stream.get_len_snd_buf()))
        }
        info
    }

    ///give the role of the TcplsConnection instance
    pub fn get_role(self) -> Role {
        self.role
    }
}
