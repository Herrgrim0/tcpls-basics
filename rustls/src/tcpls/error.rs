/// Handle errors of tcpls
#[derive(Debug)]
pub enum Error {
    /// The buffer has not enough place to accept a
    /// new TCPLS frame
    NotEnoughPlace,
    /// The buffer with the frame to read is empty
    EmptyBuffer,
    /// An unknown type has been found in the buffer
    UnknownTcplsType,
    /// An erronous stream id has been given
    StreamNotFound,
}