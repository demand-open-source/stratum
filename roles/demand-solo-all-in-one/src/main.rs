pub mod args;
mod ck_proxy;
mod healty_check;
mod jd_client;
mod translator;
use std::sync::mpsc::channel as schannel;

use tokio::sync::oneshot::channel;

async fn start_demand_session() {
    let (t, r) = channel();
    let (free_sokcet_tx, free_sokcet_rx) = channel();

    tokio::select! {
        _ = jd_client::main_jd(t) => {},
        _ = translator::main_translator(r, free_sokcet_rx) => {},
        _ = healty_check::healthy_check() => {},
    };
    let _ = free_sokcet_tx.send(());
}

#[tokio::main]
async fn main() {
    std::thread::spawn(|| {
        let (tx, rx) = schannel();
        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
            .expect("Error setting Ctrl-C handler");
        rx.recv().expect("Could not receive from channel.");
        std::process::exit(0);
    });
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    start_demand_session().await;
    ck_proxy::listen_downstream().await
}
