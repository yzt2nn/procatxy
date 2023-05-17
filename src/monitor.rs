use crate::utils;
use crossterm::{cursor, execute, terminal};
use std::io::stdout;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::sleep;
use std::time::Duration;

struct Connection {
    id: u32,
    server_name: Option<String>,
    server_port: u32,
    total_forward_traffic: f64, // 总计转发流量(KB)
    total_receive_traffic: f64, // 总计接收流量(KB)
    last_frame_total_forward_traffic: f64,
    last_frame_total_reveive_traffic: f64,
    last_timestamp: u64,
}

pub enum MonitorAction {
    ConnInit { id: u32, port: u32 },          // 监听到请求时
    ProxyOk { id: u32, server_name: String }, // 代理连接成功时
    ForwardTraffic { id: u32, length: f64 },  // 转发了多少流量
    ReceiveTraffic { id: u32, length: f64 },  // 接收了多少流量
    Heartbeat { id: u32, timestamp: u64 },    // 心跳包
}

pub struct Monitor {
    conn_list: Vec<Connection>,
    pub sender: Sender<MonitorAction>,
    receiver: Receiver<MonitorAction>,
    total_forward_traffic: f64, // 总计转发流量(KB)
    total_receive_traffic: f64, // 总计接收流量(KB)
}

impl Monitor {
    pub fn new() -> Self {
        let (sender, receiver) = channel::<MonitorAction>();
        Self {
            conn_list: Vec::<Connection>::new(),
            sender,
            receiver,
            total_forward_traffic: 0.0,
            total_receive_traffic: 0.0,
        }
    }

    fn add_conn(&mut self, conn: Connection) {
        self.conn_list.push(conn);
    }

    fn clear() {
        let mut stdout = stdout();
        execute!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();
    }

    fn draw(&mut self) {
        Monitor::clear();
        self.clear_dead_conn();

        let now = utils::get_timestamp();

        println!(
            "| waiting request... | current connection count: {0} | total: ↑{1:.3} MB, ↓{2:.3} MB | now timestamp: {3} |\n",
            self.conn_list.len(),
            self.total_forward_traffic / 1000.0,
            self.total_receive_traffic / 1000.0,
            now
        );

        for conn in &mut self.conn_list {
            let server_name = match &conn.server_name {
                Some(name) => name,
                None => "connecting proxy...",
            };

            println!(
                "| id:{0} | {1}:{2} | speed: ↑{3:.2} KB/s, ↓{4:.2} KB/s | total: ↑{5:.3} MB, ↓{6:.3} MB |",
                conn.id,
                server_name,
                conn.server_port,
                conn.total_forward_traffic - conn.last_frame_total_forward_traffic,
                conn.total_receive_traffic - conn.last_frame_total_reveive_traffic,
                conn.total_forward_traffic / 1000.0,
                conn.total_receive_traffic / 1000.0,
            );

            conn.last_frame_total_forward_traffic = conn.total_forward_traffic;
            conn.last_frame_total_reveive_traffic = conn.total_receive_traffic;

            if conn.server_name.is_some() && now - conn.last_timestamp > 10 {
                conn.last_timestamp = 0;
            }

            if conn.server_name.is_none() && now - conn.last_timestamp > 60 {
                // 60秒没连上代理，则清理连接
                conn.last_timestamp = 0;
            }
        }
        print!("\n");
    }

    fn clear_dead_conn(&mut self) {
        let mut i = 0;
        while i < self.conn_list.len() {
            if self.conn_list[i].last_timestamp == 0 {
                self.conn_list.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn find_conn_by_id(&mut self, id: u32) -> Result<&mut Connection, &'static str> {
        for conn in &mut self.conn_list {
            if conn.id == id {
                return Ok(conn);
            }
        }
        Err("not found")
    }

    pub fn start(&mut self) {
        loop {
            let action = self.receiver.try_recv();
            if let Ok(action) = action {
                match action {
                    MonitorAction::ConnInit { id, port } => {
                        self.add_conn(Connection {
                            id,
                            server_name: None,
                            server_port: port,
                            total_forward_traffic: 0.0,
                            total_receive_traffic: 0.0,
                            last_frame_total_forward_traffic: 0.0,
                            last_frame_total_reveive_traffic: 0.0,
                            last_timestamp: utils::get_timestamp(),
                        });
                        continue;
                    }
                    MonitorAction::ProxyOk { id, server_name } => {
                        if let Ok(conn) = self.find_conn_by_id(id) {
                            conn.server_name = Some(server_name);
                        }
                        continue;
                    }
                    MonitorAction::ForwardTraffic { id, length } => {
                        if let Ok(conn) = self.find_conn_by_id(id) {
                            conn.total_forward_traffic += length;
                        }
                        self.total_forward_traffic += length;
                        continue;
                    }
                    MonitorAction::ReceiveTraffic { id, length } => {
                        if let Ok(conn) = self.find_conn_by_id(id) {
                            conn.total_receive_traffic += length;
                        }
                        self.total_receive_traffic += length;
                        continue;
                    }
                    MonitorAction::Heartbeat { id, timestamp } => {
                        if let Ok(conn) = self.find_conn_by_id(id) {
                            conn.last_timestamp = timestamp;
                        }
                        continue;
                    }
                }
            }
            self.draw();
            sleep(Duration::new(1, 0));
        }
    }
}
