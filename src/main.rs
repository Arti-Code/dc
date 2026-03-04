pub mod answer;
pub mod event_handler;
pub mod offer;
use std::net::Ipv4Addr;
use std::net::IpAddr;
use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use crate::offer::process_offerer;
use crate::answer::process_answerer;

fn main() -> Result<()> {
    block_on(async_main())
}

async fn async_main() -> Result<()> {
    let sdp_modes = &[
        "OFFER",
        "ANSWER"
    ];
    let sdp_mode = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("select SDP mode").default(0).items(&sdp_modes[..]).interact().unwrap();
    let name = read_input("enter name")?;
    match sdp_mode {
        0 => {
            process_offerer(&name).await?;
        },
        // ANSWERER
        1 => {
            process_answerer(&name).await?;
        },
        _ => unreachable!(),
    }
    Ok(())
}


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