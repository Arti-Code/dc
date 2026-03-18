use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use colored::*;
use signaler::client::Client as SignalClient;
use futures::FutureExt;
use webrtc::runtime::channel;

fn main() -> Result<()> {
        block_on(async_main())
}

async fn async_main() -> Result<()> {
    let (ctrlc_tx, mut ctrlc_rx) = channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;
    display_init();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
    .default("LOGGER".to_string()).allow_empty(false).show_default(true)
    .interact_text().unwrap();

    let (_done_tx, mut done_rx) = channel::<()>(1);
    let url = "ws://yamanote.proxy.rlwy.net:25134";
    let mut signal_client = SignalClient::new(&name, url);
    signal_client.connect().await?;
    signal_client.set_logger().await?;
    println!("{}", "connected as a logger".to_string().blue().bold());
    let mut recv = signal_client.get_receiver().unwrap();
    let reading = tokio::spawn(async move {
        while let Some(log) = recv.recv().await { 
            println!("{}", (log as String).italic());
        }
    });

    futures::select! {
        _ = done_rx.recv().fuse() => {
            println!("{}", "closing".to_string().yellow().bold());
            signal_client.send_close().await?;
            reading.abort();
        }
        _ = ctrlc_rx.recv().fuse() => {
            println!("{}", "ctrl+c user closing".to_string().yellow().bold());
            signal_client.send_close().await?;
            reading.abort();
        }
    }
    Ok(())
}

fn display_init() {
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let authors = env!("CARGO_PKG_AUTHORS").to_string();
    let title = format!("--== Signaling Server Logger ==--");
    let date = "2026y".to_string();
    println!("");
    println!("{}", title.underline().bold().green());
    println!("");
    println!("{} {}", "version".to_string().yellow(), ver.yellow());
    println!("{} {}", authors.italic().cyan(), date.italic().cyan());
    println!("");
    println!("");
}