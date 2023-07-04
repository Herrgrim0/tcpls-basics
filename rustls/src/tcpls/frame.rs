use crate::tcpls::Reader;
use crate::tcpls::Codec;

enum_builder! {
    /// Tcpls frame type
    @U8
    EnumName: TcplsFrameType;
    EnumVal{
        Padding => 0x00,
        Ping => 0x01,
        Ack => 0x04
    }
}




#[derive(Clone, Debug)]
pub struct Frame {
    pub(crate) typ: TcplsFrameType,
}

impl Frame {
    pub fn encode(&self, bytes: &mut Vec<u8>) {
        self.typ.encode(bytes);
    }

    pub fn read(_typ: TcplsFrameType, _r: &mut Reader) {}

    pub fn ack_a_ping(typ: TcplsFrameType, highest_record_seq_recv: u64, conn_id: u32) -> AckFrame {
        AckFrame { typ: TcplsFrameType::Ack, highest_record_seq_recv, conn_id}
    }
}

#[derive(Clone, Debug)]
pub struct AckFrame {
    typ: TcplsFrameType,
    highest_record_seq_recv: u64,
    conn_id: u32,
}

impl AckFrame {
    fn new(highest_record_seq_recv: u64, conn_id: u32) -> AckFrame {
        Self {
            typ: TcplsFrameType::Ack,
            highest_record_seq_recv,
            conn_id,
        }
    }
}

impl Codec for AckFrame {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.append(&mut self.conn_id.to_be_bytes().to_vec());
        bytes.append(&mut self.highest_record_seq_recv.to_be_bytes().to_vec());
        self.typ.encode(bytes);
    }

    fn read(_r: &mut Reader) -> Option<Self> {
        Some(Self { typ: TcplsFrameType::Ack, highest_record_seq_recv: 0, conn_id: 0 })
    }

    fn get_encoding(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode(&mut bytes);
        bytes
    }

    fn read_bytes(bytes: &[u8]) -> Option<Self> {
        let mut reader = Reader::init(bytes);
        Self::read(&mut reader)
    }
}