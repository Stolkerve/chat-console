#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
use shared_utils::{decode_header, decode_msg, encode_msg, Msg, MSG_MAX_BYTES_SIZE};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

fn main() {
    let mut options = eframe::NativeOptions::default();
    options.always_on_top = true;
    eframe::run_native(
        "Chat console",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );

    //     tx.send().await.unwrap();
}

struct MyApp {
    tokio_runtime: runtime::Runtime,
    username: String,
    current_msg: String,
    show_chat: bool,
    spawned_client: bool,
    tx: Option<UnboundedSender<Vec<u8>>>,
    msgs: Vec<String>,
    new_msg_lock: Arc<AtomicBool>,
    new_msg: Arc<Mutex<String>>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            tokio_runtime: runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            username: "".to_owned(),
            current_msg: "".to_owned(),
            show_chat: false,
            spawned_client: false,
            tx: None,
            msgs: Vec::new(),
            new_msg_lock: Arc::new(AtomicBool::new(false)),
            new_msg: Arc::new(Mutex::new(String::new())),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.show_chat {
                ui.heading("Wellcome");
                ui.horizontal(|ui| {
                    ui.label("Your username: ");
                    ui.text_edit_singleline(&mut self.username);
                });
                if ui.button("Submit").clicked() {
                    if self.username.len() != 0 {
                        self.show_chat = true;
                    }
                }
            } else {
                if !self.spawned_client {
                    let (tx, mut rx) = unbounded_channel::<Vec<u8>>();
                    self.tx = Some(tx.clone());
                    let new_msg_lock = self.new_msg_lock.clone();
                    let new_msg = self.new_msg.clone();
                    self.tokio_runtime.spawn(async move {
                        let socket = TcpStream::connect("127.0.0.1:8000").await;
                        if socket.is_err() {
                            std::process::exit(-1);
                        }
                        let mut socket = socket.unwrap();
                        let (mut reader, mut writer) = socket.split();
                        let mut msg_len_buf = vec![0; MSG_MAX_BYTES_SIZE];
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
                                    {
                                        new_msg_lock.store(true, Ordering::SeqCst);
                                        let mut m_msg = new_msg.lock().unwrap();
                                        *m_msg = format!("[{}] {}", msg.username, msg.msg);
                                    }
                                }
                                msg = rx.recv() => {
                                    writer.write_all(&msg.unwrap()[..]).await.unwrap();
                                }
                            }
                        }
                    });
                    self.spawned_client = true;
                }

                if self.new_msg_lock.load(Ordering::SeqCst) {
                    self.new_msg_lock.store(false, Ordering::SeqCst);
                    let m_msg = self.new_msg.lock().unwrap();
                    self.msgs.push(m_msg.to_string());
                }

                // Panels
                // Chat panel
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let text_style = egui::TextStyle::Body;
                    let row_height = ui.text_style_height(&text_style);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .show_rows(ui, row_height, self.msgs.len(), |ui, _| {
                            for msg in self.msgs.iter().as_ref() {
                                ui.heading(msg);
                            }
                        });
                });
                // Input panel
                egui::TopBottomPanel::bottom("bottom_panel")
                    .resizable(false)
                    .min_height(0.0)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.horizontal(|ui| {
                                let response = ui.add_sized(
                                    ui.available_size(),
                                    egui::TextEdit::singleline(&mut self.current_msg),
                                );
                                if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter)
                                {
                                    if self.current_msg.len() != 0 && self.tx.is_some() {
                                        self.tx
                                            .as_mut()
                                            .unwrap()
                                            .send(encode_msg(&Msg {
                                                username: self.username.clone(),
                                                msg: self.current_msg.clone(),
                                            }))
                                            .unwrap();
                                        self.new_msg_lock.store(true, Ordering::SeqCst);
                                        let mut m_msg = self.new_msg.lock().unwrap();
                                        *m_msg =
                                            format!("[{}] {}", self.username, self.current_msg);
                                        self.current_msg.clear();
                                    }
                                }
                            });
                        });
                    });
            }
        });
    }
}
