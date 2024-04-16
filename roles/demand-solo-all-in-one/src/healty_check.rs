use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_tls::HttpsConnector;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use serde::Deserialize;
use tracing::{error, info, log::warn};

#[derive(Debug, Clone, Copy)]
pub enum Version {
    V1,
    V2,
}

impl Version {
    pub fn get_api_endpoint(&self, address: &str) -> hyper::Uri {
        match self {
            Version::V1 => format!(
                "https://app.dmnd.work/api/mining_data_solo?address={}",
                address
            )
            .parse::<hyper::Uri>()
            .unwrap(),
            Version::V2 => format!(
                "https://app.dmnd.work/api/mining_data_solo_2?address={}",
                address
            )
            .parse::<hyper::Uri>()
            .unwrap(),
        }
    }
}

pub async fn healthy_check(version: Version) {
    info!("Start healthy check");
    let address = std::env::var("ADDRESS").unwrap();
    // Wait 5 minutes before start healthy check so there is pleanty of time to connect to pool TP
    // and do all the initialization stuff
    tokio::time::sleep(std::time::Duration::from_secs(60 * 5)).await;
    let mut missed = 0;
    loop {
        if !is_mining(&address, version).await {
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

async fn is_mining(address: &str, version: Version) -> bool {
    let https = HttpsConnector::new();
    let client = Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);
    let url = version.get_api_endpoint(address);
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
            let mut tot_value = 0.0;
            if last_hr.is_empty() {
                warn!("No worker registerd with pool")
            } else {
                for x in &last_hr {
                    tot_value += x.value;
                    if x.value == 0.0 {
                        warn!("Pool hash rate for worker {} is {}", x.username, x.value);
                    } else {
                        info!("Pool hash rate for worker {} is {}", x.username, x.value);
                    }
                }
            }
            #[allow(clippy::needless_bool)]
            if last_hr.is_empty() || tot_value == 0.0 {
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
