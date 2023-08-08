/// Handle errors of tcpls
#[derive(Debug)]
#[derive(PartialEq)]
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
    /// when converting a slice of byte to an unsigned
    /// of a certain size, the number of bytes given is wrong
    BadSliceLength,
    /// the record has not a size between 0 and MAX_RECORD_SIZE
    UnexpectedRecordSize,
}