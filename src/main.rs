pub mod answer;
pub mod event_handler;
pub mod offer;
pub mod util;
use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use colored::*;
use crate::util::{get_local_ip, read_input};
use crate::offer::process_offerer;
use crate::answer::process_answerer;

fn main() -> Result<()> {
    block_on(async_main())
}

async fn async_main() -> Result<()> {
    display_init();
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
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

fn display_init() {
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let authors = env!("CARGO_PKG_AUTHORS").to_string();
    let title = format!("-=WebRTC Client=-");
    println!("{}", title.underline().bold().green());
    println!("{}{}", "version".to_string().bright_green(), ver.bright_green());
    println!("{}{}", "by".to_string().italic().cyan(), authors.italic().cyan());
}