mod forward;
mod monitor;
mod proxy;
mod utils;

use std::{env, sync::Arc, sync::Mutex};

use forward::Listener;
use monitor::Monitor;
use proxy::ProxyConfig;

fn main() {
    let args: Vec<String> = env::args().collect();
    let proxy_type = &args[1];
    let ip_port: Vec<&str> = args[2].split(':').collect();
    let proxy_config;
    if proxy_type.to_lowercase() == "socks5" {
        proxy_config = ProxyConfig::Socks5 {
            ip: String::from(ip_port[0]),
            port: ip_port[1].parse().unwrap(),
        };
    } else {
        panic!("unsupported proxy type")
    }

    let mut monitor = Monitor::new();

    let global_id = Arc::new(Mutex::new(0));

    let mut https_listener = Listener::new(443, proxy_config.clone(), Arc::clone(&global_id));
    https_listener.set_monitor_sender(monitor.sender.clone());
    https_listener.start();

    let mut http_listener = Listener::new(80, proxy_config.clone(), Arc::clone(&global_id));
    http_listener.set_monitor_sender(monitor.sender.clone());
    http_listener.start();

    monitor.start();
}
