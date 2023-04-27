#![allow(missing_docs)]
use crate::msgs::codec::{Codec, Reader};

/// Implementation of TCPLS for Rust in rutls
/// 
/// 

#[derive(Clone, Debug)]
struct Frame {
    typ: TcplsFrameType,
}

impl Frame {
    fn encode(&self, bytes: &mut Vec<u8>) {
        self.typ.encode(bytes);
    }

    fn read(typ: TcplsFrameType, _r: &mut Reader) -> Self {
        Self{typ}
    }
}

enum_builder! {
    /// Tcpls frame type
    @U8
    EnumName: TcplsFrameType;
    EnumVal{
        Padding => 0x00,
        Ping => 0x01
        //Stream => 0x02,
        //ACK => 0x04,
        //NewToken => 0x05,
        //ConnectionReset => 0x06,
        //NewAddress => 0x07,
        //RemoveAddress => 0x08,
        //StreamChange => 0x09
    }
}

#[derive(Clone, Debug)]
enum TcplsFrame {
    Padding(Frame),
    Ping(Frame),
}

impl TcplsFrame {
    fn get_type(&self) -> TcplsFrameType {
        match *self {
            Self::Padding(_) => TcplsFrameType::Padding,
            Self::Ping(_) => TcplsFrameType::Ping,
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
        }
    }

    fn read(r: &mut Reader) -> Option<Self> {
        let typ = TcplsFrameType::read(r)?;
        let len = u16::read(r)? as usize;
        let mut sub = r.sub(len)?;

        let frm = match typ {
            TcplsFrameType::Padding => Self::Padding(Frame::read(typ, &mut sub)),
            TcplsFrameType::Ping => Self::Ping(Frame::read(typ, &mut sub)),
            TcplsFrameType::Unknown(_) => todo!(),
        };

        if sub.any_left() {
            None
        } else {
            Some(frm)
        }
    }
}