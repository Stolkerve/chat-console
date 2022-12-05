#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
use shared_utils::{decode_header, decode_msg, encode_msg, MSG_MAX_BYTES_SIZE, MsgType, MsgDataType, MsgRoleType, Msg, encode_str};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Chat console",
        options,
        Box::new(|_cc| Box::new(ChatConsole::default())),
    );
}

struct ChatConsole {
    tokio_runtime: runtime::Runtime,
    username: String,
    current_msg_input: String,
    show_chat: bool,
    spawned_client: bool,
    tx: Option<UnboundedSender<Vec<u8>>>,
    msgs: Vec<Msg>,
    new_msg_lock: Arc<AtomicBool>, // Check if the new msg have something and push to msgs
    new_msg: Arc<Mutex<Option<Msg>>>,   // new msg across threads
}

impl Default for ChatConsole {
    fn default() -> Self {
        Self {
            tokio_runtime: runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            username: "".to_owned(),
            current_msg_input: "".to_owned(),
            show_chat: false,
            spawned_client: false,
            tx: None,
            msgs: Vec::new(),
            new_msg_lock: Arc::new(AtomicBool::new(false)),
            new_msg: Arc::new(Mutex::new(None)),
        }
    }
}

impl ChatConsole {
    fn login_ui(&mut self, ui: &mut egui::Ui) {
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
    }

    fn chat_ui(&mut self, ui: &mut egui::Ui) {
        if !self.spawned_client {
            self.spawned_client();
        }

        if self.new_msg_lock.load(Ordering::SeqCst) {
            self.new_msg_lock.store(false, Ordering::SeqCst);
            let new_msg = self.new_msg.lock().unwrap().take();
            self.msgs.push(new_msg.unwrap());
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
                        ui.horizontal_wrapped(|ui| {
                            let mut role_color = None;
                            match msg.role {
                                MsgRoleType::Server => {
                                    role_color = Some(egui::Color32::from_rgb(255, 191, 0));
                                },
                                MsgRoleType::User => {
                                    if msg.username == self.username {
                                        role_color = Some(egui::Color32::from_rgb(0, 128, 0));
                                    }
                                    else {
                                        role_color = Some(egui::Color32::from_rgb(0, 143, 143));
                                    }
                                },
                            }
                            match &msg.data {
                                MsgDataType::Text(msg_text) => {
                                    ui.colored_label(role_color.unwrap(), format!("[{}]", msg.username));
                                    ui.label(msg_text)
                                },
                            }
                        });
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
                            egui::TextEdit::singleline(&mut self.current_msg_input),
                        );
                        if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter)
                        {
                            if self.current_msg_input.len() != 0 && self.tx.is_some() {
                                let msg = MsgType::Msg(Msg {
                                        username: self.username.clone(),
                                        data: MsgDataType::Text(self.current_msg_input.clone()),
                                        role: MsgRoleType::User
                                    });
                                self.tx
                                    .as_mut()
                                    .unwrap()
                                    .send(encode_msg(&msg))
                                    .unwrap();
                                self.new_msg_lock.store(true, Ordering::SeqCst);
                                let mut new_msg = self.new_msg.lock().unwrap();
                                if let MsgType::Msg(msg) = msg {
                                    *new_msg = Some(msg);
                                }
                                self.current_msg_input.clear();
                            }
                        }
                    });
                });
            });
    }

    fn spawned_client(&mut self) {
        let (tx, mut rx) = unbounded_channel::<Vec<u8>>();
        self.tx = Some(tx.clone());

        let new_msg_lock = self.new_msg_lock.clone();
        let new_msg_mutex = self.new_msg.clone();
        let username = self.username.clone();

        self.tokio_runtime.spawn(async move {
            let socket = TcpStream::connect("127.0.0.1:8000").await;
            if socket.is_err() {
                std::process::exit(-1);
            }
            let mut socket = socket.unwrap();
            let (mut reader, mut writer) = socket.split();
            let mut incoming_msg_len_buf = vec![0; MSG_MAX_BYTES_SIZE];

            writer.write_all(&encode_str(&username)).await.unwrap();

            loop {
                tokio::select! {
                    bytes_readed = reader.read(&mut incoming_msg_len_buf) => {
                        let bytes_readed = bytes_readed.unwrap();
                        if bytes_readed == 0 {
                            std::process::exit(-1);
                        }
                        let incoming_msg_len = decode_header(&incoming_msg_len_buf[..]) as usize;
                        let mut buf = vec![0; incoming_msg_len];
                        reader.read(&mut buf).await.unwrap();

                        new_msg_lock.store(true, Ordering::SeqCst);
                        let incomig_msg = decode_msg(&buf).unwrap();
                        let mut new_msg = new_msg_mutex.lock().unwrap();
                        *new_msg = Some(incomig_msg); 
                    }
                    msg = rx.recv() => {
                        writer.write_all(&msg.unwrap()[..]).await.unwrap();
                    }
                }
            }
        });
        self.spawned_client = true;
    }
}

impl eframe::App for ChatConsole {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.show_chat {
                self.login_ui(ui);
            } else {
                self.chat_ui(ui);
            }
        });
    }
}