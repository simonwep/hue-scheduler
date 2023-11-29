use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
pub struct Config {
    pub bridge_ip: IpAddr,
    pub bridge_username: String,
    pub ping_interval: Duration,
    pub reachability_window: Duration,
    pub home_latitude: f64,
    pub home_longitude: f64,
}

pub fn load_config() -> Config {
    if dotenv::dotenv().is_err() {
        println!("No .env file found");
    }

    let bridge_username = env::var("BRIDGE_USERNAME").expect("BRIDGE_USERNAME missing");
    let bridge_raw_addr = env::var("BRIDGE_IP").expect("BRIDGE_IP missing");
    let bridge_ip = IpAddr::from_str(bridge_raw_addr.as_str()).expect("failed to parse BRIDGE_IP");

    let ping_interval = Duration::from_millis(
        env::var("PING_INTERVAL")
            .expect("PING_INTERVAL missing")
            .parse::<u64>()
            .expect("failed to parse INTERVAL"),
    );

    let reachability_window = Duration::from_millis(
        env::var("REACHABILITY_WINDOW")
            .expect("REACHABILITY_WINDOW missing")
            .parse::<u64>()
            .expect("failed to parse REACHABILITY_WINDOW"),
    );

    let home_latitude = env::var("HOME_LATITUDE")
        .expect("HOME_LATITUDE missing")
        .parse::<f64>()
        .expect("failed to parse HOME_LATITUDE");

    let home_longitude = env::var("HOME_LONGITUDE")
        .expect("HOME_LONGITUDE missing")
        .parse::<f64>()
        .expect("failed to parse HOME_LONGITUDE");

    Config {
        bridge_ip,
        bridge_username,
        ping_interval,
        reachability_window,
        home_latitude,
        home_longitude,
    }
}
