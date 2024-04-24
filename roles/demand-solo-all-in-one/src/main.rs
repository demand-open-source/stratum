pub mod args;
mod healty_check;
mod jd_client;
mod sv1_proxy;
mod translator;
use clap::Parser;
use std::sync::mpsc::channel as schannel;
use tracing::info;

use healty_check::Version;
use sv1_proxy::Upstream;
use tokio::sync::oneshot::channel;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args_ {
    #[arg(
        short,
        long,
        help = "When proxy is runned with this flag it just redirect each sv1 conection that\nreceive to dmnd solo pool. When is runnned without, it aggregate all the sv1\nconnection from downstream to the sv2 demand endpoint with job declaration.\nTo use this last option is needed a TP listening on localhost:8442.\nIt always fallback to ckpool if hashrate read from demand solo is 0 for more than 3 minutes. (+5 at start)",
        default_value = "false"
    )]
    without_jd: bool,
    #[arg(short, long, help = "Valid bitcoin address for the block reward")]
    address: String,
    #[arg(
        short,
        long,
        help = "Port number where the proxy should listen for downstream sv1 connection",
        default_value = "34255"
    )]
    downstream_port: u16,
}

async fn start_demand_session(have_block_declaration: bool, port: u16) {
    let (t, r) = channel();
    let (free_sokcet_tx, free_sokcet_rx) = channel();

    if have_block_declaration {
        tokio::select! {
            _ = jd_client::main_jd(t) => {},
            _ = translator::main_translator(r, free_sokcet_rx) => {},
            _ = healty_check::healthy_check(Version::V2) => {},
        };
    } else {
        tokio::select! {
            _ = sv1_proxy::listen_downstream(Upstream::new_demand(),port, Some(free_sokcet_rx),0) => {},
            _ = healty_check::healthy_check(Version::V1) => {},
        };
    };
    let _ = free_sokcet_tx.send(());
}

#[tokio::main]
async fn main() {
    let args = Args_::parse();
    std::thread::spawn(|| {
        let (tx, rx) = schannel();
        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
            .expect("Error setting Ctrl-C handler");
        rx.recv().expect("Could not receive from channel.");
        std::process::exit(0);
    });
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    std::env::set_var("ADDRESS", args.address);
    std::env::set_var("DOWNSTREAM_PORT", args.downstream_port.to_string());
    info!("Starting demand session");
    start_demand_session(!args.without_jd, args.downstream_port).await;
    info!("Starting ck pool session");
    sv1_proxy::listen_downstream(Upstream::new_ck(), args.downstream_port, None, 15).await
}
