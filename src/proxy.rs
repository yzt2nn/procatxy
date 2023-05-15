use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct Socks5 {}

impl Socks5 {
    pub fn connect(
        ip: &str,
        port: u32,
        server_name: &str,
        server_port: u32,
    ) -> io::Result<TcpStream> {
        let mut proxy_stream = TcpStream::connect(format!("{ip}:{port}"))?;

        // 客户端请求认证，只使用无认证模式
        proxy_stream.write(&[0x05, 0x01, 0x00]).unwrap();

        // 收到回应
        let mut buf = [0x00; 512];
        proxy_stream.read(&mut buf).unwrap();

        let server_name = server_name.as_bytes();

        // 构建命令请求 0x01 - CONNECT
        let mut data = vec![0x05, 0x01, 0x00, 0x03, server_name.len() as u8];
        for b in server_name.into_iter() {
            data.push(b.clone());
        }
        if server_port == 443 {
            // port: 443
            data.push(0x01);
            data.push(0xbb);
        } else if server_port == 80 {
            // port: 80
            data.push(0x00);
            data.push(0x50);
        } else {
            panic!("unsupported port")
        }

        proxy_stream.write(&data).unwrap();

        let length = proxy_stream.read(&mut buf).unwrap();
        if length > 2 {
            let msg = &buf[..length];
            if msg[1] == 0x00 {
                proxy_stream
                    .set_read_timeout(Some(Duration::new(0, 100000000)))
                    .unwrap();
                return Ok(proxy_stream);
            } else {
                panic!("connect failed, response: {}", msg[1]);
            }
        } else {
            panic!("connect failed")
        }
    }
}
