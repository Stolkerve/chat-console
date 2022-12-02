use std::io::Write;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

use shared_utils::{decode_msg, decode_header, encode_msg, encode_str, read_from_socket, Msg, MSG_MAX_BYTES_SIZE};

fn input(mgs: &str) -> String {
    print!("{}", mgs);
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input
}

#[tokio::main]
async fn main() {
    let mut username = input("Enter your username: ");
    loop {
        if username.len() != 0 && username.len() < 20 {
            username.pop(); // remove the end line char
            break;
        }
        username = input("Enter your username: ");
    }

    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(32);

    let s_username = username.clone();
    tokio::spawn(async move {
        let socket = TcpStream::connect("127.0.0.1:8000").await;
        if socket.is_err() {
            std::process::exit(-1);
        }
        let mut socket = socket.unwrap();
        let (mut reader, mut writer) = socket.split();

        writer.write_all(&encode_str(&s_username)[..]).await.unwrap();

        let mut msg_len_buf = vec![0; MSG_MAX_BYTES_SIZE];

        let wellcome_buf = read_from_socket(&mut reader, &mut msg_len_buf).await;
        println!("[Server] {}", String::from_utf8(wellcome_buf).unwrap());

        loop {
            tokio::select! {
                bytes_readed = reader.read(&mut msg_len_buf) => {
                    let bytes_readed = bytes_readed.unwrap();
                    if bytes_readed == 0 {
                        std::process::exit(-1);
                    }
                    let msg_len = decode_header(&msg_len_buf[..]) as usize;
                    let mut buf = vec![0; msg_len];
                    reader.read(&mut buf).await.unwrap();
                    let msg = decode_msg(&String::from_utf8(buf).unwrap());
                    println!("[{}] {}", msg.username, msg.msg);
                }
                msg = rx.recv() => {
                    writer.write_all(&msg.unwrap()[..]).await.unwrap();
                }
            }
        }
    });

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        input.pop();
        tx.send(encode_msg(&Msg{username: username.clone(), msg: input})).await.unwrap();
    }
}
