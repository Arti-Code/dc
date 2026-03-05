pub mod answer;
pub mod event_handler;
pub mod offer;
pub mod util;
use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use colored::*;
use crate::offer::process_offerer;
use crate::answer::process_answerer;

fn main() -> Result<()> {
    block_on(async_main())
}

async fn async_main() -> Result<()> {
    display_init();
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    let sdp_modes = &[
        "ANSWER",
        "OFFER"
    ];
    let sdp_mode = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("select SDP mode").default(0)
        .items(&sdp_modes[..]).interact().unwrap();
    match sdp_mode {
        0 => {
            let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
            .default("USER".to_string()).allow_empty(false).show_default(true)
            .interact_text().unwrap();
            let target: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter target")
            .default("ROBOT".to_string()).allow_empty(false).show_default(true)
            .interact_text().unwrap();
            process_offerer(&name, &target).await?;
        },
        1 => {
            let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
            .default("ROBOT".to_string()).allow_empty(false).show_default(true)
            .interact_text().unwrap();
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
    let date = "2026y".to_string();
    println!("");
    println!("{}", title.underline().bold().green());
    println!("");
    println!("{} {}", "version".to_string().bright_green(), ver.bright_green());
    println!("{} {}", authors.italic().cyan(), date.italic().cyan());
    println!("");
    println!("");
}