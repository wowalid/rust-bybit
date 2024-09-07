use bybit::ws::future;
use bybit::ws::response::FuturePublicResponse;
use bybit::KlineInterval;
use bybit::WebSocketApiClient;

fn main() {
    env_logger::init();

    let mut client = WebSocketApiClient::future_inverse().build();

    let symbol = "ETHUSD";

    client.subscribe_orderbook(symbol, future::OrderbookDepth::Level1);
    client.subscribe_orderbook(symbol, future::OrderbookDepth::Level50);
    client.subscribe_trade(symbol);
    client.subscribe_ticker(symbol);
    client.subscribe_kline(symbol, KlineInterval::Min1);
    client.subscribe_liquidation(symbol);


}
