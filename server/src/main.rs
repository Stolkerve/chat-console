use shared_utils::{decode_header, encode_bytes, decode_msg_type, MSG_MAX_BYTES_SIZE, MsgType, encode_msg, Msg, MsgRoleType, MsgDataType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::broadcast,
};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("Couldn't bind server");

    let (tx, _) = broadcast::channel::<(String, Vec<u8>)>(32);

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();

            let mut msg_len_buf = vec![0; MSG_MAX_BYTES_SIZE];

            // wellcome msg
            reader.read(&mut msg_len_buf).await.unwrap();
            let mut username = vec![0; decode_header(&msg_len_buf) as usize];
            reader.read(&mut username).await.unwrap();
            tx.send((addr.to_string(), encode_msg(
                &MsgType::Msg(Msg{
                    username: "Server".to_owned(),
                    role: MsgRoleType::Server,
                    data: MsgDataType::Text(format!("Wellcome {}", String::from_utf8(username).unwrap()))
                })
            ))).unwrap();

            loop {
                tokio::select! {
                    bytes_readed = reader.read(&mut msg_len_buf) => {
                        let n = bytes_readed.unwrap();
                        if n == 0 {
                            println!("Peer {:?} disconected", addr);
                            break;
                        }
                        let len = decode_header(&msg_len_buf[..]);

                        let mut buf = vec![0; len as usize];
                        reader.read(&mut buf).await.unwrap();
                        let msg = decode_msg_type(&buf).unwrap();
                        match msg {
                            MsgType::Msg(_) => {
                                tx.send((addr.to_string(), encode_bytes(buf))).unwrap();
                            },
                            MsgType::Login(_) => todo!(),
                            MsgType::Register(_) => todo!(),
                            _ => {}
                        }
                    },
                    msg = rx.recv() => {
                        let (sender_addr, msg) = msg.unwrap();
                        if addr.to_string() != sender_addr {
                            writer.write_all(&msg[..]).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}