use std::fs::File;
use std::io::BufReader;

use crate::models::config::Config;

pub async fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Open the file in read-only mode with buffer.
    let file: File = File::open("./config.json")?;
    let reader: BufReader<File> = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let config: Config = serde_json::from_reader(reader)?;

    Ok(config)
}
