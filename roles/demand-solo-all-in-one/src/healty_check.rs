use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::Client,
    rt::TokioExecutor,
};
use serde::Deserialize;
use tracing::error;

pub async fn healthy_check() {
    let address = std::env::var("ADDRESS").unwrap();
    // Wait 5 minuts before start healthy check so there is pleanty of time to connect to pool TP
    // and do all the initialization stuff
    tokio::time::sleep(std::time::Duration::from_secs(60 * 5)).await;
    let mut missed = 0;
    loop {
        if !is_mining(&address).await {
            missed += 1;
        } else {
            missed = 0
        }
        if missed > 2 {
            error!("Can not get mining metrics since 3 mins, fallback to ckpool");
            error!("If you are doing block declaration a possible solution could be to restart the bitcoin node and when ready restart also this proxy");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn is_mining(address: &str) -> bool {
    let https = HttpsConnector::new();
    let url = format!(
        "https://app.dmnd.work/api/mining_data_solo_2?address={}",
        address
    )
    .parse::<hyper::Uri>()
    .unwrap();
    let client = Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);
    let response = client
        .get(url)
        .await
        .unwrap_or_else(|e| panic!("Web backend error {}", e));
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("Web backend unavailable")
        .to_bytes();
    if status.is_success() {
        let body = std::str::from_utf8(body.as_ref()).unwrap_or_else(|e| {
            panic!("Invalid response {}", e);
        });
        if let Ok(last_hr) = serde_json::from_str::<Vec<Stats>>(body) {
            #[allow(clippy::needless_bool)]
            if last_hr.is_empty() || last_hr[0].value == 0.0 {
                false
            } else {
                true
            }
        } else {
            panic!("Invalid response {:?}", body);
        }
    } else {
        panic!("Web backend unavailable");
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stats {
    pub username: String,
    pub secondary: String,
    pub upstream: String,
    pub timestamp: String,
    pub value: f64,
}
