use std::net::{IpAddr, Ipv4Addr};
use anyhow::Result;


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
