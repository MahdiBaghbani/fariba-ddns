use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client, Response, StatusCode};
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::{task, time, time::Interval};
use tracing::{debug, error, info};
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

mod providers;
mod settings;

use crate::providers::arvancloud::arvan_update_dns;
use crate::providers::cloudflare::models::ZoneResponse;
use crate::settings::models::Cloudflare;
use crate::settings::settings;

fn main() {
    // loads the .env file from the current directory or parents.
    dotenvy::dotenv_override().ok();

    // load settings from toml file.
    settings::init();

    // setup logging.
    let filter: EnvFilter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .parse_lossy(settings().log.level.clone());
    let filtered_layer = fmt::layer().with_level(true).with_filter(filter);
    tracing_subscriber::registry().with(filtered_layer).init();

    info!("⚙️ Settings have been loaded.");
    debug!("{:#?}", settings());

    run();
}

#[tokio::main]
async fn run() {
    let forever = task::spawn(async move {
        let duration: Duration = Duration::from_secs(settings().update.interval * 60);
        let mut interval: Interval = time::interval(duration);

        {
            let interval: String = settings().update.interval.to_string();
            info!("🕰️ Updating IPv4 (A) records every {interval} minutes");
        }

        loop {
            interval.tick().await;
            let ip_v4: Option<Ipv4Addr> = get_public_ip_v4().await;

            if let Some(ip) = ip_v4 {
                info!("Public 🧩 IPv4 detected: {:?}", ip);

                for cloudflare in settings().cloudflare.clone() {
                    if !cloudflare.api_token.is_empty() && cloudflare.api_token != "api_token_here"
                    {
                        let client: Client = cloudflare_prepare_reqwest_client(&cloudflare)
                            .await
                            .unwrap();

                        cloudflare_get_zone_info(&cloudflare, &client).await;
                    } else {
                        error!("🚩 Cloudflare API token is not set!");
                    }
                }
            } else {
                info!("🧩 IPv4 not detected");
            }
        }
    });

    forever.await.expect("forever panicked");
}

async fn get_public_ip_v4() -> Option<Ipv4Addr> {
    // attempt to get an IP address.
    public_ip::addr_v4().await
}

async fn cloudflare_prepare_reqwest_client(cloudflare: &Cloudflare) -> Option<Client> {
    // create headers.
    let mut headers: HeaderMap = HeaderMap::new();

    // consider marking security-sensitive headers with `set_sensitive`.
    let bearer_token: String = format!("Bearer {}", &cloudflare.api_token);
    let mut auth_value: HeaderValue = HeaderValue::from_str(&bearer_token).unwrap();
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    // get a client builder.
    Client::builder().default_headers(headers).build().ok()
}

async fn cloudflare_get_zone_info(cloudflare: &Cloudflare, client: &Client) {
    let url: String = format!(
        "https://api.cloudflare.com/client/v4/zones/{}",
        cloudflare.zone_id
    );

    let response: Option<Response> = client.get(url).send().await.ok();

    if let Some(response) = response {
        let status: StatusCode = response.status();
        debug!("Response status code: {}", status.as_str());

        if response.status() == StatusCode::OK {
            let zone_data: ZoneResponse = response.json::<ZoneResponse>().await.unwrap();

            println!("{:#?}", zone_data);
        }
    }
}

async fn do_something1() -> Result<(), Box<dyn std::error::Error>> {
    let ipv4_finder: &str = "https://ident.me";

    let ipv4_address: Option<String> = reqwest::get(ipv4_finder).await?.json().await.ok();

    if let Some(ipv4_address) = ipv4_address {
        let arvan_api_secret: &str = "Apikey f3d8d8ef-bb3a-5b3f-bed2-2f175c4528cb";
        arvan_update_dns(
            &ipv4_address,
            "azadehafzar.ir",
            "fariba-ddns",
            arvan_api_secret,
        )
        .await
        .expect("TODO: panic message");
    }

    Ok(())
}
