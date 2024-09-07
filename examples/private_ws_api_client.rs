use bybit::ws::response::PrivateResponse;
use bybit::WebSocketApiClient;
use std::env;

fn main() {
    env_logger::init();

    let api_key: String = env::var("BYBIT_API_KEY").unwrap();
    let secret: String = env::var("BYBIT_SECRET").unwrap();

    let mut client = WebSocketApiClient::private()
        .testnet()
        .build_with_credentials(api_key, secret);

    client.subscribe_position();
    client.subscribe_execution();
    client.subscribe_order();
    client.subscribe_wallet();
    client.subscribe_greek();

 
}
