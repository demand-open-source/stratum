use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tracing::{error, info};
use v1::{json_rpc::Message, methods::Client2Server, Method};

pub async fn listen_downstream() {
    let bitcoin_address = std::env::var("ADDRESS")
        .expect("Env var ADDRESS must be set with the solo pool coinbase address");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let socket = TcpListener::bind("0.0.0.0:34255")
        .await
        .expect("Impossible to bin ck proxy on 0.0.0.0:34255");
    let (connection, address) = socket
        .accept()
        .await
        .expect("Listen address no more available");
    info!("{:?} connected", address);
    start_proxy(connection, address.to_string(), bitcoin_address.clone()).await;
}

async fn start_proxy(dw: TcpStream, address: String, bitcoin_address: String) {
    let up = TcpStream::connect("solo.ckpool.org:3333")
        .await
        .expect("Ck pool not available");
    let (up_read, mut up_send) = up.into_split();
    let (dw_read, mut dw_send) = dw.into_split();
    {
        let address = address.clone();
        tokio::task::spawn(async move {
            let mut reader = BufReader::new(up_read);
            let mut received = String::new();
            let address = address.clone();
            loop {
                reader.read_line(&mut received).await.unwrap_or_else(|e| {
                    error!("Ck pool dropped {:?}", e);
                    std::process::abort();
                });
                if received.is_empty() {
                    error!("Downstream dropped {}", address);
                    break;
                }
                info!("> {} {}", &address, received);
                let to_send = received.clone().into_bytes();
                if let Err(e) = dw_send.write(to_send.as_ref()).await {
                    error!("Downstream dropped {} {}", address, e);
                    break;
                }
                received.clear();
            }
        });
    }
    {
        let address = address.clone();
        tokio::task::spawn(async move {
            let mut reader = BufReader::new(dw_read);
            let mut received = String::new();
            let address = address.clone();

            while reader.read_line(&mut received).await.is_ok() {
                if let Ok(Ok(parsed)) = serde_json::from_str::<Message>(&received)
                    .map(TryInto::<Method>::try_into)
                {
                    if let Method::Client2Server(Client2Server::Authorize(a)) = parsed {
                        received = format!(
                            r#"{{"id": {}, "method": "mining.authorize", "params": ["{}", "x"]}}"#,
                            a.id, bitcoin_address
                        );
                        received.push('\n');
                    }
                    let to_send = received.clone().into_bytes();
                    info!("< {} {}", &address, received);
                    if let Err(e) = up_send.write(to_send.as_ref()).await {
                        error!("Ck pool dropped {:?}",e);
                        std::process::abort();
                    }
                    received.clear();
                } else {
                    break;
                }
            }
        });
    }
}
