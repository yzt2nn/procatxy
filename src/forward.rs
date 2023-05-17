use crate::{
    monitor::MonitorAction,
    proxy::{self, ProxyConfig},
    utils,
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
    time::Duration,
};

pub struct Listener {
    port: u32,
    global_id: Arc<Mutex<u32>>,
    monitor_sender: Option<Sender<MonitorAction>>,
    proxy_config: ProxyConfig,
}

impl Listener {
    pub fn new(port: u32, proxy_config: ProxyConfig, global_id: Arc<Mutex<u32>>) -> Self {
        Self {
            port,
            monitor_sender: None,
            global_id,
            proxy_config,
        }
    }

    pub fn set_monitor_sender(&mut self, monitor_sender: Sender<MonitorAction>) {
        self.monitor_sender = Some(monitor_sender);
    }

    pub fn start(&self) {
        let port: u32 = self.port;
        let sender = self.monitor_sender.clone();
        let global_id = Arc::clone(&self.global_id);
        let proxy_config = self.proxy_config.clone();

        thread::spawn(move || {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
            for stream in listener.incoming() {
                if let Ok(client_stream) = stream {
                    let mut id = global_id.lock().unwrap();
                    *id += 1;
                    let current_id = *id;
                    let sender = sender.clone();
                    let proxy_config = proxy_config.clone();
                    thread::spawn(move || {
                        handle_conn(current_id, client_stream, port, proxy_config, sender);
                    });
                }
            }
        });
    }
}

fn send_to_monitor(monitor_sender: &Option<Sender<MonitorAction>>, monitor_action: MonitorAction) {
    if let Some(sender) = monitor_sender {
        sender.send(monitor_action).unwrap();
    }
}

fn handle_conn(
    id: u32,
    mut client_stream: TcpStream,
    server_port: u32,
    proxy_config: ProxyConfig,
    monitor_sender: Option<Sender<MonitorAction>>,
) {
    send_to_monitor(
        &monitor_sender,
        MonitorAction::ConnInit {
            id,
            port: server_port,
        },
    );

    let mut last_heartbeat_timestamp = 0;
    let mut msg = [0x00; 8192];

    let size = client_stream.read(&mut msg).unwrap();
    if size == 0 {
        panic!("client disconnected");
    }
    let server_name = if server_port == 443 {
        utils::get_server_name_from_hello_client_message(&msg).unwrap()
    } else {
        utils::get_server_name_from_http_request_message(&msg).unwrap()
    };

    // 获取server name后，开始连接proxy
    let proxy_stream;
    match proxy_config {
        ProxyConfig::Socks5 { ip, port } => {
            proxy_stream = proxy::Socks5::connect(&ip, port, &server_name, server_port)
        }
    }
    let mut proxy_stream = proxy_stream.unwrap_or_else(|_| {
        // 失败则发送0心跳包，告知监视器可以移除
        send_to_monitor(
            &monitor_sender,
            MonitorAction::Heartbeat { id, timestamp: 0 },
        );
        panic!("connect proxy failed")
    });
    send_to_monitor(&monitor_sender, MonitorAction::ProxyOk { id, server_name });

    client_stream
        .set_read_timeout(Some(Duration::new(0, 100000000)))
        .unwrap();

    // 转发第一个数据包
    proxy_stream.write(&msg[..size]).unwrap();
    send_to_monitor(
        &monitor_sender,
        MonitorAction::ForwardTraffic {
            id,
            length: size as f64 / 1000.0,
        },
    );

    loop {
        let receive_length = receive_all_to_client(&mut client_stream, &mut proxy_stream).unwrap();

        if receive_length > 0 {
            send_to_monitor(
                &monitor_sender,
                MonitorAction::ReceiveTraffic {
                    id,
                    length: receive_length as f64 / 1000.0,
                },
            );
        }

        let forward_length = forward_all_to_proxy(&mut client_stream, &mut proxy_stream).unwrap();

        if forward_length > 0 {
            send_to_monitor(
                &monitor_sender,
                MonitorAction::ForwardTraffic {
                    id,
                    length: forward_length as f64 / 1000.0,
                },
            );
        }

        // 大约每5秒发送一次心跳包
        let now = utils::get_timestamp();
        if now - last_heartbeat_timestamp >= 5 {
            send_to_monitor(
                &monitor_sender,
                MonitorAction::Heartbeat { id, timestamp: now },
            );
            last_heartbeat_timestamp = now;
        }
    }
}

fn forward_all_to_proxy<'a>(
    client_stream: &mut TcpStream,
    proxy_stream: &mut TcpStream,
) -> Result<usize, &'a str> {
    let mut buf = [0x00; 4096];
    let mut total_length = 0;
    loop {
        let rev_length = match client_stream.read(&mut buf) {
            Ok(length) => length,
            Err(_) => return Ok(total_length),
        };
        if rev_length == 0 {
            return Err("client disconnect");
        }
        proxy_stream.write(&buf[..rev_length]).unwrap();
        total_length += rev_length;
    }
}

fn receive_all_to_client<'a>(
    client_stream: &mut TcpStream,
    proxy_stream: &mut TcpStream,
) -> Result<usize, &'a str> {
    let mut buf = [0x00; 4096];
    let mut total_length = 0;
    loop {
        let rev_length = match proxy_stream.read(&mut buf) {
            Ok(length) => length,
            Err(_) => return Ok(total_length),
        };
        if rev_length == 0 {
            return Err("proxy disconnect");
        }
        client_stream.write(&buf[..rev_length]).unwrap();
        total_length += rev_length;
    }
}
