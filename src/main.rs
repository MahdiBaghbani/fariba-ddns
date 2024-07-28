use std::time::Duration;

use tokio::time::Interval;
use tokio::{task, time};

use models::config::Config;
use providers::arvancloud::arvan_update_dns;
use config::config::read_config;

mod models;
mod providers;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let forever = task::spawn(async {
        // Read the JSON contents of the file as an instance of `User`.
        let config: Config = read_config().await.expect("failed to read config");
        let mut interval: Interval = time::interval(Duration::from_secs(config.frequency));

        loop {
            interval.tick().await;
            do_something().await.expect("TODO: panic message");
        }
    });

    forever.await.expect("TODO: panic message");

    Ok(())
}

async fn do_something() -> Result<(), Box<dyn std::error::Error>> {
    let ipv4_finder: &str = "https://ident.me";

    let ipv4_address: Option<String> = reqwest::get(ipv4_finder).await?.text().await.ok();

    if let Some(ipv4_address) = ipv4_address {
        let arvan_api_secret: &str = "Apikey f3d8d8ef-bb3a-5b3f-bed2-2f175c4528cb";
        arvan_update_dns(
            &*ipv4_address,
            "azadehafzar.ir",
            "fariba-ddns",
            arvan_api_secret,
        )
        .await
        .expect("TODO: panic message");
    }

    Ok(())
}
