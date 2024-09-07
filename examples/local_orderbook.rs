use bybit::ws::response::{OrderbookItem, SpotPublicResponse};
use bybit::ws::spot;
use bybit::WebSocketApiClient;
use std::io::{self, Write};

struct OwnedOrderBookItem(String, String);

impl<'a> From<&OrderbookItem<'a>> for OwnedOrderBookItem {
    fn from(value: &OrderbookItem) -> Self {
        OwnedOrderBookItem(value.0.to_owned(), value.1.to_owned())
    }
}

fn main() {
    let mut client = WebSocketApiClient::spot().build();

    let symbol = "ETHUSDT";

    client.subscribe_trade(symbol);
    client.subscribe_orderbook(symbol, spot::OrderbookDepth::Level50);

    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout);

    let mut latest_price: String = String::new();
    let mut direction = "△";
    let mut asks: Vec<OwnedOrderBookItem> = Vec::new();
    let mut bids: Vec<OwnedOrderBookItem> = Vec::new();

  
}
