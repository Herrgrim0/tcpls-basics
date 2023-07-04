#![allow(missing_docs)]
use crate::msgs::codec::{Codec, Reader};

mod frame;
pub(crate) use frame::*;
use log::debug;


/// Implementation of TCPLS for Rust in rustls



#[derive(Clone, Debug)]
pub enum TcplsFrame {
    Padding(Frame),
    Ping(Frame),
    Ack(AckFrame),
}


/*#[derive(Clone, Debug)]
pub struct StreamFrame {
    stream_data: Vec<u8>,
    len: u16,
    stream_id: u32,
    fin: u8,
    typ: TcplsFrameType,
}

impl Codec for StreamFrame {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.stream_data[..self.len as usize]);
        bytes.push(self.len as u8);
        bytes.push(self.stream_id as u8);
        bytes.push(self.fin);
    }

    fn read(r: &mut Reader) -> Option<Self> {
        if TcplsFrameType::Stream.get_u8() == u8::read(r)? {
            let typ = TcplsFrameType::Stream;
        } else {
            return None;
        }

        
        
        
        let bytes = r.take(1)?;
        let mut fin = [0;1];
        fin.clone_from_slice(bytes);

        let bytes = r.take(4)?;
        let mut stream_id = [0; 4];
        stream_id.clone_from_slice(bytes);

        let bytes = r.take(2)?;
        let mut len = [0; 2];
        len.clone_from_slice(bytes);
        let len2 = u16::read(r)?;

        return None

    }
}*/


impl TcplsFrame {
    fn get_type(&self) -> TcplsFrameType {
        match *self {
            Self::Padding(_) => TcplsFrameType::Padding,
            Self::Ping(_) => TcplsFrameType::Ping,
            Self::Ack(_) => TcplsFrameType::Ack,
            //Self::Stream(_) => TcplsFrameType::Stream,
        }
    }

}

impl Codec for TcplsFrame {
    fn encode(&self, bytes: &mut Vec<u8>) {
        self.get_type().encode(bytes);

        let mut sub: Vec<u8> = Vec::new();
        
        match *self {
            Self::Padding(ref r) => r.encode(&mut sub),
            Self::Ping(ref r) => r.encode(&mut sub),
            Self::Ack(ref r) => r.encode(&mut sub),
            //Self::Stream(ref r) => r.encode(&mut sub),
        }

        (sub.len() as u16).encode(bytes);
        bytes.append(&mut sub);
    }

    fn read(r: &mut Reader) -> Option<Self> {
        let typ = TcplsFrameType::read(r)?;
        let len = u16::read(r)? as usize;
        let mut sub = r.sub(len)?;

        let frm = match typ {
            TcplsFrameType::Padding => debug!("Padding")/*Self::Padding(Frame::read(typ, &mut sub))*/,
            TcplsFrameType::Ping => debug!("Ping")/*Self::Ping(Frame::read(typ, &mut sub))*/,
            TcplsFrameType::Ack => debug!("Ack") /*Self::Ack(AckFrame::read(&mut sub)?)*/,
            //TcplsFrameType::Stream => Self::Stream(StreamFrame::read(&mut sub)?),
            TcplsFrameType::Unknown(_) => todo!(),
        };

        if sub.any_left() {
            None
        } else {
            Some(Self::Ping(Frame {typ}))
        }
    }
}

/// Enqueue borrowed fragments of (version, typ, payload) which
/// are no longer than max_frag onto the `out` deque. with tcpls protocol in it
pub fn fragment_tcpls<'a>(
    payload: &'a [u8],
    control_buf: &'a[u8],
) {
    let max_frag = 16384;
    let total_len = payload.len() + control_buf.len();
}

/// create a tcpls records that will "not" be fragmented because it as the
/// maximum accepatble size.
pub fn create_record(payload: &[u8], max_size: usize, bytes: &mut Vec<u8>, tls_seq: u64) {
    //bytes.append(payload);
    for _i in 0..max_size {
        TcplsFrame::Ping( Frame { typ: TcplsFrameType::Ping }).encode(bytes);
    }
}


/// Manage a TCPLS record (set of frames)
pub struct Tcpls {
    max_size: u16,
    curr_size: u16,
    read_data: Vec<u8>,
    write_data: Vec<u8>,
}

impl Tcpls {

    pub fn new() -> Tcpls {
        Self { max_size: 16384, curr_size: 0, read_data: Vec::new(), write_data: Vec::new() }
    }
    // update the vector containing frames without
    fn update(&self) -> u16 {
        todo!();
    }

    fn process_tcpls_record(&self) -> u16 {
        todo!();
    }
}