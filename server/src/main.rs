use shared_utils::{decode_header, encode_str, MSG_MAX_BYTES_SIZE, encode_bytes};
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
                        tx.send((addr.to_string(), encode_bytes(buf))).unwrap();
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
