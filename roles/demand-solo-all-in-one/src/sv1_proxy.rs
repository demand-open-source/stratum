use std::sync::Arc;

use rand::distributions::{Alphanumeric, DistString};
use roles_logic_sv2::utils::Mutex;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::oneshot::Receiver,
};
use tracing::{error, info};
use v1::{json_rpc::Message, methods::Client2Server, Method};

#[derive(Debug, Clone)]
pub enum Upstream {
    CkPool(String),
    DemandSolo(String),
}

impl From<&Upstream> for String {
    fn from(value: &Upstream) -> Self {
        match value {
            Upstream::CkPool(s) => s.clone(),
            Upstream::DemandSolo(s) => s.clone(),
        }
    }
}

impl Upstream {
    pub async fn connect(&self) -> TcpStream {
        let addr: String = self.into();
        TcpStream::connect(&addr)
            .await
            .unwrap_or_else(|_| panic!("Upstream at {} not available", addr))
    }
    pub fn new_ck() -> Self {
        Self::CkPool("solo.ckpool.org:3333".to_string())
    }
    pub fn new_demand() -> Self {
        Self::DemandSolo("mining.dmnd.work:1000".to_string())
    }

    pub fn authorize(&self, id: u64, bitcoin_address: &str, user: &str) -> String {
        let mut message = match self {
            Upstream::CkPool(_) => format!(
                r#"{{"id": {}, "method": "mining.authorize", "params": ["{}", "x"]}}"#,
                id, bitcoin_address
            ),
            Upstream::DemandSolo(_) => {
                format!(
                    r#"{{"id": {}, "method": "mining.authorize", "params": ["{}", "{}"]}}"#,
                    id, user, bitcoin_address
                )
            }
        };
        message.push('\n');
        message
    }
}

pub async fn listen_downstream(
    upstream: Upstream,
    port: u16,
    kill: Option<Receiver<()>>,
    timeout: u64,
) {
    let bitcoin_address = std::env::var("ADDRESS")
        .expect("Env var ADDRESS must be set with the solo pool coinbase address");
    tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;
    let socket = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Impossible to bind sv1 proxy");
    info!("Sv1 proxy listening on {}", port);
    let connections = Arc::new(Mutex::new(Vec::new()));
    let handler = {
        let connections = connections.clone();
        tokio::task::spawn(async move {
            while let Ok((connection, address)) = socket.accept().await {
                info!("{:?} connected", address);
                let bitcoin_address = bitcoin_address.clone();
                let upstream = upstream.clone();
                let handler = tokio::task::spawn(async move {
                    start_proxy(connection, address.to_string(), bitcoin_address, upstream).await
                });
                connections.safe_lock(|s| s.push(handler)).unwrap();
            }
        })
    };
    if let Some(kill) = kill {
        let _ = kill.await;
        handler.abort();
        connections
            .safe_lock(|s| {
                for handler in s {
                    handler.abort();
                }
            })
            .unwrap();
    } else {
        let _ = handler.await;
    }
}

async fn start_proxy(dw: TcpStream, address: String, bitcoin_address: String, upstream: Upstream) {
    let up = upstream.connect().await;
    info!("Start sv1 proxy for {:?}", upstream);

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
        let _ = tokio::task::spawn(async move {
            let mut reader = BufReader::new(dw_read);
            let mut received = String::new();
            let address = address.clone();
            let user_name = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

            while reader.read_line(&mut received).await.is_ok() {
                if let Ok(Ok(parsed)) =
                    serde_json::from_str::<Message>(&received).map(TryInto::<Method>::try_into)
                {
                    if let Method::Client2Server(Client2Server::Authorize(ref a)) = parsed {
                        received = upstream.authorize(a.id, &bitcoin_address, &user_name);
                    }
                    if let Method::Client2Server(Client2Server::Submit(mut s)) = parsed {
                        s.user_name = match upstream {
                            Upstream::CkPool(_) => bitcoin_address.clone(),
                            Upstream::DemandSolo(_) => user_name.clone(),
                        };
                        let message: v1::json_rpc::Message = s.into();
                        received = match serde_json::to_string(&message) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Error serializing sumbit message {:?}", e);
                                received
                            }
                        };
                        received.push('\n');
                    }
                    let to_send = received.clone().into_bytes();
                    info!("< {} {}", &address, received);
                    if let Err(e) = up_send.write(to_send.as_ref()).await {
                        match upstream {
                            Upstream::CkPool(_) => {
                                error!("CkPool dropped {:?}", e);
                                break;
                            }
                            Upstream::DemandSolo(_) => {
                                error!("Demand sv1 dropped {:?}", e);
                                break;
                            }
                        }
                    }
                    received.clear();
                } else {
                    break;
                }
            }
        })
        .await;
    }
}
