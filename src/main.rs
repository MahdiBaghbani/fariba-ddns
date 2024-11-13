use std::error::Error;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client, Response, StatusCode};
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

mod providers;
mod settings;

use providers::cloudflare::models::ZoneResponse;
use settings::models::Settings;
use settings::models::{Cloudflare, ConfigManager};

#[tokio::main]
async fn main() {
    // loads the .env file from the current directory or parents.
    dotenvy::dotenv_override().ok();

    // Create ConfigManager and wrap it in Arc
    let config: Arc<ConfigManager> =
        Arc::new(ConfigManager::new().expect("Failed to initialize configuration"));

    // setup logging.
    let log_level: String = config.get_log_level().await;

    let filter: EnvFilter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .parse_lossy(log_level)
        .add_directive("hyper_util=error".parse().unwrap())
        .add_directive("reqwest=error".parse().unwrap())
        .add_directive("trust_dns_proto=error".parse().unwrap())
        .add_directive("hyper_system_resolver=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_level(true)
        .init();

    info!("⚙️ Settings have been loaded.");

    // Run the main application logic
    run(config).await.expect("Failed to run the application");
}

async fn run(config_manager: Arc<ConfigManager>) -> Result<(), Box<dyn Error>> {
    info!(
        "🕰️ Updating IPv4 (A) records every {} seconds",
        config_manager.get_update_interval().await
    );

    let mut previous_ip: Option<Ipv4Addr> = None;

    loop {
        let update_interval: u64;
        let cloudflare_settings: Vec<Cloudflare>;

        {
            let settings: RwLockReadGuard<Settings> = config_manager.get_settings().await;
            update_interval = settings.update.interval;
            cloudflare_settings = settings.cloudflare.clone();
        }

        // Get the public IPv4 address
        match get_public_ip_v4().await {
            Some(ip) => {
                if Some(ip) != previous_ip {
                    info!("Public 🧩 IPv4 detected: {}", ip);
                    previous_ip = Some(ip);

                    // Process Cloudflare updates
                    if let Err(e) = process_cloudflare_updates(cloudflare_settings, ip).await {
                        error!("Error updating Cloudflare records: {}", e);
                    }
                } else {
                    debug!("🧩 IPv4 address unchanged");
                }
            }
            None => {
                warn!("🧩 IPv4 not detected");
            }
        }

        // Sleep for the update interval duration
        tokio::time::sleep(Duration::from_secs(update_interval)).await;
    }
}

async fn process_cloudflare_updates(
    cloudflare_settings: Vec<Cloudflare>,
    _ip: Ipv4Addr,
) -> Result<(), Box<dyn Error>> {
    let futures = cloudflare_settings.into_iter().map(|cloudflare| {
        tokio::spawn(async move {
            if !cloudflare.api_token.is_empty() && cloudflare.api_token != "your_api_token_here" {
                let client = cloudflare_prepare_reqwest_client(&cloudflare).unwrap();

                if let Err(e) = cloudflare_get_zone_info(&cloudflare, &client).await {
                    error!("Error updating zone info for {}: {}", cloudflare.name, e);
                }
            } else {
                error!("🚩 Cloudflare API token is not set for {}", cloudflare.name);
            }
        })
    });

    let _results = FuturesUnordered::from_iter(futures)
        .collect::<Vec<_>>()
        .await;

    Ok(())
}

fn cloudflare_prepare_reqwest_client(cloudflare: &Cloudflare) -> Result<Client, Box<dyn Error>> {
    // Create headers.
    let mut headers: HeaderMap = HeaderMap::new();

    // Mark security-sensitive headers with `set_sensitive`.
    let bearer_token: String = format!("Bearer {}", &cloudflare.api_token);
    let mut auth_value: HeaderValue = HeaderValue::from_str(&bearer_token)?;
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    // Build the client.
    let client: Client = Client::builder().default_headers(headers).build()?;

    Ok(client)
}

async fn cloudflare_get_zone_info(
    cloudflare: &Cloudflare,
    client: &Client,
) -> Result<(), Box<dyn Error>> {
    let url: String = format!(
        "https://api.cloudflare.com/client/v4/zones/{}",
        cloudflare.zone_id
    );

    // Build the request to get DNS records
    let response: Response = client.get(&url).send().await?;

    if response.status() != StatusCode::OK {
        let error_text: String = response.text().await?;
        error!(
            "Failed to get zone info for {}: {}",
            cloudflare.name, error_text
        );
        return Ok(());
    }

    let zone_data: ZoneResponse = response.json::<ZoneResponse>().await.unwrap();
    debug!("zone data {:#?}", zone_data);

    if !zone_data.success {
        error!(
            "Failed to get zone info for {}, success field is not true",
            cloudflare.name
        );
        return Ok(());
    }

    if zone_data.result.status != "active" {
        error!(
            "Failed to get zone info for {}, zone is not active",
            cloudflare.name
        );
        return Ok(());
    }

    debug!("Successfully updated DNS records for {}", cloudflare.name);

    Ok(())
}

async fn cloudflare_update_dns_data(
    cloudflare: &Cloudflare,
    client: &Client,
) -> Result<(), Box<dyn Error>> {
    todo!()
}

async fn get_public_ip_v4() -> Option<Ipv4Addr> {
    // attempt to get an IP address.
    public_ip::addr_v4().await
}
