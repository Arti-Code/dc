use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use colored::*;
use dc::offer::process_offerer;
use dc::answer::process_answerer;

fn main() -> Result<()> {
    block_on(async_main())
}

async fn async_main() -> Result<()> {
    /* let (ctrlc_tx, mut ctrlc_rx) = mpsc::channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?; */

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
        1 => {
            let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
            .default("USER".to_string()).allow_empty(false).show_default(true)
            .interact_text().unwrap();
            let target: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter target")
            .default("ROBOT".to_string()).allow_empty(false).show_default(true)
            .interact_text().unwrap();
            loop { 
                match process_offerer(&name, &target).await? {
                    true => continue,
                    false => break,
                }
            }
        },
        0 => {
            let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
            .default("ROBOT".to_string()).allow_empty(false).show_default(true)
            .interact_text().unwrap();
            let restart: bool = false;
            loop {
                match process_answerer(&name, restart).await? {
                    true => continue,
                    false => break,
                }
            }
        },
        _ => unreachable!(),
    }
    Ok(())
}

fn display_init() {
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let authors = env!("CARGO_PKG_AUTHORS").to_string();
    let title = format!("--== WebRTC Client ==--");
    let date = "2026y".to_string();
    println!("");
    println!("{}", title.underline().bold().green());
    println!("");
    println!("{} {}", "version".to_string().yellow(), ver.yellow());
    println!("{} {}", authors.italic().cyan(), date.italic().cyan());
    println!("");
    println!("");
}