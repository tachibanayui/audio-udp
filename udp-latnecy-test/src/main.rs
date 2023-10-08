pub mod std_send;

use serde::{Deserialize, Serialize};
use std::{error::Error, fs};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
enum Mode {
    Send,
    Receive,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ConfigArgs {
    implementaion: String,
    mode: Mode,
    host: String,
    /// packet_size = timestamp + random bit
    packet_size: u64, 
    total: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = fs::read_to_string("./udp_test_settings.yaml")?;
    let x: ConfigArgs = serde_yaml::from_str(&config)?;
    dbg!(x.clone());

    match x.implementaion.as_str() {
        "std-send" => (),
        _ => (),
    }

    Ok(())
}
