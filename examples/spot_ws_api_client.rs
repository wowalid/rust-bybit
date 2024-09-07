use std::future::Future;
use std::pin::Pin;

use bybit::error::BybitError;
use bybit::ws::response::SpotPublicResponse;
use bybit::ws::response::SpotPublicResponseArg;
use bybit::ws::spot;
use bybit::KlineInterval;
use bybit::WebSocketApiClient;
use futures::future::BoxFuture;

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut client = WebSocketApiClient::spot().build();

    let symbol = "ETHUSDT";
    let lt_symbol = "BTC3SUSDT";

    client.subscribe_orderbook(symbol, spot::OrderbookDepth::Level1);
    client.subscribe_orderbook(symbol, spot::OrderbookDepth::Level50);
    client.subscribe_trade(symbol);
    client.subscribe_ticker(symbol);
    client.subscribe_kline(symbol, KlineInterval::Min1);
    client.subscribe_lt_kline(lt_symbol, KlineInterval::Min5);
    client.subscribe_lt_ticker(lt_symbol);
    client.subscribe_lt_nav(lt_symbol);

    let callback: Box<dyn FnMut(SpotPublicResponseArg) -> Pin<Box<dyn Future<Output = Result<(), BybitError>> + Send>> + Send> =
    Box::new(|res: SpotPublicResponseArg| {
        Box::pin(async move {
            // Process `res` here
            println!("Received: ");
            Ok(())
        })
    });

    // Assuming `client` is properly defined elsewhere and `run` matches the expected signature
    if let Err(e) = client.run(callback).await {
        eprintln!("Error: {e}");
    }
}
