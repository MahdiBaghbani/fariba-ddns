use std::collections::HashMap;

use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::{Client, Response, StatusCode};
use strfmt::strfmt;

pub mod models;

use crate::providers::arvancloud::models::{
    ArvanDNSData, ArvanDNSRecord, ArvanIPFilterMode, ArvanIPv4Record,
};

pub async fn arvan_update_dns(
    ipv4_address: &str,
    domain_name: &str,
    sub_domain: &str,
    api_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut vars: HashMap<String, &str> = HashMap::new();
    vars.insert("domain_name".to_string(), domain_name);
    vars.insert("sub_domain".to_string(), sub_domain);

    let template: &str = "https://napi.arvancloud.ir/cdn/4.0/domains/{domain_name}/dns-records";
    let api_endpoint: String = strfmt(&template, &vars).unwrap();
    println!("{api_endpoint}");

    let template: &str = "?type=a&search={sub_domain}";
    let query_substring: String = strfmt(template, &vars).unwrap();
    println!("{query_substring}");

    let api_query_endpoint: String = format!("{}{}", api_endpoint, query_substring);
    println!("{api_query_endpoint}");

    // create headers.
    let mut headers: HeaderMap = HeaderMap::new();

    // Consider marking security-sensitive headers with `set_sensitive`.
    let mut auth_value: HeaderValue = HeaderValue::from_str(api_token).unwrap();
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    // get a client builder
    let client: Client = Client::builder().default_headers(headers).build()?;

    let response: Response = client.get(api_query_endpoint).send().await?;

    let status = response.status();
    let status_str = status.as_str();
    println!("{status_str}");

    match response.status() {
        StatusCode::OK => {
            let dns_record: ArvanDNSData = response.json::<ArvanDNSData>().await?;

            if dns_record.data.len() == 0 {
                let response: Response = arvan_push_to_dns(
                    &client,
                    "post",
                    &*api_endpoint,
                    vars.get("sub_domain").unwrap(),
                    ipv4_address,
                )
                .await
                .unwrap();

                match response.status() {
                    StatusCode::CREATED => println!("successfully created dns record"),
                    StatusCode::UNAUTHORIZED => println!("unauthorized access"),
                    StatusCode::NOT_FOUND => println!("not found"),
                    StatusCode::UNPROCESSABLE_ENTITY => println!("unprocessable entity"),
                    _ => println!("fuck off 1"),
                }
            } else {
                let dns_record: &ArvanDNSRecord = dns_record.data.get(0).unwrap();
                let dns_record_value: &ArvanIPv4Record = dns_record.value.get(0).unwrap();
                let dns_record_ip: &String = &dns_record_value.ip;

                if !dns_record_ip.eq(&ipv4_address) {
                    let url: String =
                        format!("{}/{}", api_endpoint, dns_record.id.as_ref().unwrap());

                    let response: Response = arvan_push_to_dns(
                        &client,
                        "put",
                        &*url,
                        vars.get("sub_domain").unwrap(),
                        ipv4_address,
                    )
                    .await
                    .unwrap();

                    match response.status() {
                        StatusCode::OK => println!("successfully updated dns record"),
                        StatusCode::UNAUTHORIZED => println!("unauthorized access"),
                        StatusCode::NOT_FOUND => println!("not found"),
                        StatusCode::UNPROCESSABLE_ENTITY => println!("unprocessable entity"),
                        _ => println!("fuck off 2"),
                    }
                } else {
                    println!("not updating dns record, it has not been changed yet.");
                }
            }
        }
        StatusCode::UNAUTHORIZED => println!("unauthorized"),
        StatusCode::NOT_FOUND => println!("not found"),
        _ => println!("fuck off 3 "),
    }

    Ok(())
}

async fn arvan_push_to_dns(
    client: &Client,
    method: &str,
    url: &str,
    name: &str,
    ipv4_address: &str,
) -> Result<Response, Box<dyn std::error::Error>> {
    let dns_record = ArvanDNSRecord {
        id: None,
        r#type: "a".to_string(),
        name: name.to_string(),
        value: vec![ArvanIPv4Record {
            ip: ipv4_address.to_string(),
            port: None,
            weight: 100,
            original_weight: None,
            country: "".to_string(),
        }],
        ttl: 120,
        cloud: false,
        upstream_https: "default".to_string(),
        ip_filter_mode: ArvanIPFilterMode {
            count: "single".to_string(),
            order: "none".to_string(),
            geo_filter: "none".to_string(),
        },
    };

    let response: Response;

    match method {
        "post" => response = client.post(url).json(&dns_record).send().await?,
        "put" => response = client.put(url).json(&dns_record).send().await?,
        _ => response = client.post(url).json(&dns_record).send().await?,
    }

    return Ok(response);
}
