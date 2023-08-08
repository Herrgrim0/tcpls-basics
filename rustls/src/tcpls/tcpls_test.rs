use crate::{tcpls::utils::{conversion, constant::{STREAM_FRAME_FIN, STREAM_FRAME, MAX_RECORD_SIZE}}, rand};

use super::{stream::TcplsStreamBuilder, utils::constant::MAX_STREAM_DATA_SIZE};

// TEST conversion function
// the random numbers for slc_3 have been chosen
// by running a random.randint in python in the following range: 
// for small value functions: [uXX::MIN, uXX::MAX/2]
// for big value functions: [uXX:MAX/2, uXX::MAX]
// for u16, random.org has been used
#[test]
fn test_slice_to_u16_small_value() {

    let slc_1 = [0x00_u8, 0x00];
    let slc_2 = [0x00_u8, 0x01];
    let slc_3 = u16::to_be_bytes(18904);

    assert_eq!(conversion::slice_to_u16(&slc_1).unwrap(), u16::MIN);
    assert_eq!(conversion::slice_to_u16(&slc_2).unwrap(), u16::MIN + 1);
    assert_eq!(conversion::slice_to_u16(&slc_3).unwrap(), 18904_u16);
    
}

#[test]
fn test_slice_to_u16_big_value() {
    let slc_1 = [0xFF_u8, 0xFF];
    let slc_2 = [0xFF_u8, 0xFE];
    let slc_3 = u16::to_be_bytes(42503);

    assert_eq!(conversion::slice_to_u16(&slc_1).unwrap(), u16::MAX);
    assert_eq!(conversion::slice_to_u16(&slc_2).unwrap(), u16::MAX - 1);
    assert_eq!(conversion::slice_to_u16(&slc_3).unwrap(), 42503_u16)
}

#[test]
fn test_slice_to_u32_small_value() {

    let slc_1 = [0x00_u8, 0x00, 0x00, 0x00];
    let slc_2 = [0x00_u8, 0x00, 0x00, 0x01];
    let slc_3 = u32::to_be_bytes(1775951801);

    assert_eq!(conversion::slice_to_u32(&slc_1).unwrap(), u32::MIN);
    assert_eq!(conversion::slice_to_u32(&slc_2).unwrap(), u32::MIN + 1);
    assert_eq!(conversion::slice_to_u32(&slc_3).unwrap(), 1775951801_u32);
    
}

#[test]
fn test_slice_to_u32_big_value() {
    let slc_1 = [0xFF_u8, 0xFF, 0xFF, 0xFF];
    let slc_2 = [0xFF_u8, 0xFF, 0xFF, 0xFE];
    let slc_3 = u32::to_be_bytes(4294967295);

    assert_eq!(conversion::slice_to_u32(&slc_1).unwrap(), u32::MAX);
    assert_eq!(conversion::slice_to_u32(&slc_2).unwrap(), u32::MAX - 1);
    assert_eq!(conversion::slice_to_u32(&slc_3).unwrap(), 4294967295_u32);
}

#[test]
fn test_slice_to_u64_small_value() {

    let slc_1 = [0x00_u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let slc_2 = [0x00_u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
    let slc_3 = u64::to_be_bytes(172516785778717348);

    assert_eq!(conversion::slice_to_u64(&slc_1).unwrap(), u64::MIN);
    assert_eq!(conversion::slice_to_u64(&slc_2).unwrap(), u64::MIN + 1);
    assert_eq!(conversion::slice_to_u64(&slc_3).unwrap(), 172516785778717348_u64);
    
}

#[test]
fn test_slice_to_u64_big_value() {
    let slc_1 = [0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let slc_2 = [0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE];
    let slc_3 = u64::to_be_bytes(17733336027062709243);

    assert_eq!(conversion::slice_to_u64(&slc_1).unwrap(), u64::MAX);
    assert_eq!(conversion::slice_to_u64(&slc_2).unwrap(), u64::MAX - 1);
    assert_eq!(conversion::slice_to_u64(&slc_3).unwrap(), 17733336027062709243_u64);
}

#[test]
fn test_stream_creation_max_frame_size() {
    let mut builder = TcplsStreamBuilder::new(2);
    let mut data = rand::random_vec(MAX_STREAM_DATA_SIZE).unwrap();
    builder.add_data(&data);

    let mut stream = builder.build();

    data.extend_from_slice(&u16::to_be_bytes(MAX_STREAM_DATA_SIZE as u16));
    data.extend_from_slice(&u64::to_be_bytes(0));
    data.extend_from_slice(&u32::to_be_bytes(2));
    data.push(STREAM_FRAME_FIN);

    assert_eq!(stream.create_data_frame(MAX_RECORD_SIZE).unwrap(), data);
}

#[test]
fn test_stream_creation_min_frame_size() {
    let mut builder = TcplsStreamBuilder::new(2);
    let mut data = rand::random_vec(0).unwrap_or_default();
    builder.add_data(&data);

    let mut stream = builder.build();

    data.extend_from_slice(&u16::to_be_bytes(0));
    data.extend_from_slice(&u64::to_be_bytes(0));
    data.extend_from_slice(&u32::to_be_bytes(2));
    data.push(STREAM_FRAME_FIN);

    assert_eq!(stream.create_data_frame(MAX_RECORD_SIZE).as_deref(), None);
}

#[test]
fn test_stream_creation_random_frame_size() {
    let mut builder = TcplsStreamBuilder::new(2);
    let mut data = rand::random_vec(65).unwrap();
    builder.add_data(&data);

    let mut stream = builder.build();

    data.extend_from_slice(&u16::to_be_bytes(65)); //len
    data.extend_from_slice(&u64::to_be_bytes(0)); //offset
    data.extend_from_slice(&u32::to_be_bytes(2)); //stream id
    data.push(STREAM_FRAME_FIN);

    assert_eq!(stream.create_data_frame(MAX_RECORD_SIZE).unwrap(), data);
}

// test a stream with a bigger size than the max size of a tcpls frame
#[test]
fn test_stream_creation_more_than_frame_size() {
    let mut builder = TcplsStreamBuilder::new(2);
    let size_1 = MAX_STREAM_DATA_SIZE;
    let size_2 = MAX_STREAM_DATA_SIZE/2;
    let mut data_1 = rand::random_vec(size_1).unwrap();
    let mut data_2 = rand::random_vec(size_2).unwrap();
    builder.add_data(&data_1);
    builder.add_data(&data_2);

    let mut stream = builder.build();

    data_1.extend_from_slice(&u16::to_be_bytes(MAX_STREAM_DATA_SIZE as u16)); //len
    data_1.extend_from_slice(&u64::to_be_bytes(0)); //offset
    data_1.extend_from_slice(&u32::to_be_bytes(2)); //stream id
    data_1.push(STREAM_FRAME);

    assert_eq!(stream.create_data_frame(MAX_RECORD_SIZE).unwrap(), data_1);

    data_2.extend_from_slice(&u16::to_be_bytes(size_2 as u16)); //len
    data_2.extend_from_slice(&u64::to_be_bytes(size_1 as u64)); //offset
    data_2.extend_from_slice(&u32::to_be_bytes(2)); //stream id
    data_2.push(STREAM_FRAME_FIN);

    assert_eq!(stream.create_data_frame(MAX_RECORD_SIZE).unwrap(), data_2);
}

