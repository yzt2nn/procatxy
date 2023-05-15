use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use crate::{proxy, utils};

pub fn handle_https_request(mut client_stream: TcpStream) {
    let mut msg = [0x00; 1024];
    loop {
        let size = client_stream.read(&mut msg).unwrap();
        if size == 0 {
            break;
        }

        let server_name = utils::get_server_name_from_hello_client_message(&msg).unwrap();
        println!("server name: {server_name}");

        // 获取server name后，开始连接Socks5
        let mut proxy_stream =
            proxy::Socks5::connect("192.168.198.138", 9909, &server_name, 443).unwrap();

        client_stream
            .set_read_timeout(Some(Duration::new(0, 100000000)))
            .unwrap();

        // 转发第一个数据包
        proxy_stream.write(&msg[..size]).unwrap();

        loop {
            let receive_length =
                receive_all_to_client(&mut client_stream, &mut proxy_stream).unwrap();

            if receive_length > 0 {
                println!("total:receive_length:{receive_length}");
            }

            let forward_length =
                forward_all_to_proxy(&mut client_stream, &mut proxy_stream).unwrap();

            if forward_length > 0 {
                println!("total:forward_length:{forward_length}");
            }
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
        if rev_length < 4096 {
            break;
        }
    }
    Ok(total_length)
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
            return Err("disconnect");
        }
        client_stream.write(&buf[..rev_length]).unwrap();
        total_length += rev_length;
        if rev_length < 4096 {
            break;
        }
    }
    Ok(total_length)
}
