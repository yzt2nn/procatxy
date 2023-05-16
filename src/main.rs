mod forward;
mod monitor;
mod proxy;
mod utils;

use std::{sync::Arc, sync::Mutex};

use forward::Listener;
use monitor::Monitor;

fn main() {
    let mut monitor = Monitor::new();

    let global_id = Arc::new(Mutex::new(0));

    let mut https_listener = Listener::new(443, Arc::clone(&global_id));
    https_listener.set_monitor_sender(monitor.sender.clone());
    https_listener.start();

    let mut http_listener = Listener::new(80, Arc::clone(&global_id));
    http_listener.set_monitor_sender(monitor.sender.clone());
    http_listener.start();

    monitor.start();
}
