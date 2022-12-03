use shared_utils::{decode_header, encode_str, read_from_socket, MSG_MAX_BYTES_SIZE};
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

    let (tx, _) = broadcast::channel::<(String, String)>(32);

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();

            let mut msg_len_buf = vec![0; MSG_MAX_BYTES_SIZE];

            // let wellcome_buff = read_from_socket(&mut reader, &mut msg_len_buf).await;
            // let wellcome = format!("Wellcome {}", String::from_utf8(wellcome_buff).unwrap());
            // tx.send((addr.to_string(), wellcome)).unwrap();
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
                        tx.send((addr.to_string(), String::from_utf8(buf).unwrap())).unwrap();
                    },
                    msg = rx.recv() => {
                        let (sender_addr, msg) = msg.unwrap();
                        if addr.to_string() != sender_addr {
                            writer.write_all(&encode_str(&msg)[..]).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}
