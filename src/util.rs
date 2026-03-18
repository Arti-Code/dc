use std::net::{IpAddr, Ipv4Addr};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};


pub fn get_local_ip() -> IpAddr {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = socket.local_addr()
        && let IpAddr::V4(ip) = addr.ip()
    {
        ip.into()
    } else {
        Ipv4Addr::new(127, 0, 0, 1).into()
    }
}

pub fn read_input(label: &str) -> Result<String> {
    println!("{label}: ");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}

pub fn must_read_stdin() -> Result<String> {
    let mut line = String::new();

    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}

pub fn encode(b: &str) -> String {
    BASE64_STANDARD.encode(b)
}

/// decode decodes the input from base64
/// It can optionally unzip the input after decoding
pub fn decode(s: &str) -> Result<String> {
    let b = BASE64_STANDARD.decode(s)?;
    let s = String::from_utf8(b)?;
    Ok(s)
}