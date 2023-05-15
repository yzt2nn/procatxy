use std::{net::TcpListener, thread};

mod forward;
mod proxy;
mod utils;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:443").unwrap();
    println!("start..");
    for stream in listener.incoming() {
        if let Ok(local_stream) = stream {
            println!("new request!!!!!!");
            thread::spawn(|| {
                forward::handle_https_request(local_stream);
            });
        }
    }
}
