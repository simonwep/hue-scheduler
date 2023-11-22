use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
pub struct Config {
    pub bridge_ip: IpAddr,
    pub bridge_username: String,
    pub interval: Duration,
}

pub fn load_config() -> Config {
    if dotenv::dotenv().is_err() {
        println!("No .env file found");
    }

    let bridge_username = env::var("BRIDGE_USERNAME").expect("BRIDGE_USERNAME missing");
    let bridge_raw_addr = env::var("BRIDGE_IP").expect("BRIDGE_IP missing");
    let bridge_ip = IpAddr::from_str(bridge_raw_addr.as_str()).expect("failed to parse BRIDGE_IP");

    let interval = Duration::from_millis(
        env::var("INTERVAL")
            .expect("INTERVAL missing")
            .parse::<u64>()
            .expect("failed to parse INTERVAL"),
    );

    Config {
        bridge_ip,
        bridge_username,
        interval,
    }
}
