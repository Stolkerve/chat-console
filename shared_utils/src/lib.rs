use serde::{Deserialize, Serialize};

use tokio::{io::AsyncReadExt, net::tcp::ReadHalf};

pub const MSG_MAX_BYTES_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Serialize, Deserialize, Debug)]
pub struct Msg {
    pub username: String,
    pub msg: String,
}

pub fn encode_str(str: &String) -> Vec<u8> {
    let mut buf = Vec::new();

    let mut offset: u8 = 0;
    let msg_size = str.len();
    buf.reserve(MSG_MAX_BYTES_SIZE + msg_size);

    // writing the msg size
    for _ in 0..MSG_MAX_BYTES_SIZE {
        buf.push(((msg_size >> offset) & 0xFF) as u8);
        offset += 8;
    }

    buf.splice(MSG_MAX_BYTES_SIZE.., str.bytes());

    buf
}

pub fn encode_msg(msg: &Msg) -> Vec<u8> {
    let mut buf = Vec::new();
    let serialized = serde_json::to_string(msg).unwrap();

    let mut offset: u8 = 0;
    let msg_size = serialized.len();
    buf.reserve(MSG_MAX_BYTES_SIZE + msg_size);

    // writing the msg size
    for _ in 0..MSG_MAX_BYTES_SIZE {
        buf.push(((msg_size >> offset) & 0xFF) as u8);
        offset += 8;
    }

    buf.splice(MSG_MAX_BYTES_SIZE.., serialized.bytes());

    buf
}

pub fn decode_header(data: &[u8]) -> u32 {
    let mut offset = 0;
    let mut value: u32 = 0;

    for i in 0..MSG_MAX_BYTES_SIZE {
        value |= u32::from(data[i]) << offset;
        offset += 8;
    }

    return value;
}

pub fn decode_msg(data: &String) -> Msg {
    serde_json::from_str(data).unwrap()
}

pub async fn read_from_socket(reader: &mut ReadHalf<'_>, mut msg_len_buf: &mut Vec<u8>) -> Vec<u8> {
    reader.read(&mut msg_len_buf).await.unwrap();
    let msg_len = decode_header(&msg_len_buf[..]);
    let mut buf = vec![0; msg_len as usize];
    reader.read(&mut buf).await.unwrap();

    buf
}
