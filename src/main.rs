use lazy_static::lazy_static;
use openlimits::{
    binance::{BinanceParameters, BinanceWebsocket},
    coinbase::client::websocket::CoinbaseWebsocket,
    coinbase::CoinbaseParameters,
    exchange_ws::{ExchangeWs, OpenLimitsWs},
    model::{
        websocket::{
            OpenLimitsWebSocketMessage::{OrderBook, OrderBookDiff},
            Subscription::OrderBookUpdates,
            WebSocketResponse::Generic,
        },
        AskBid,
    },
};
use rust_decimal::Decimal;
use std::io::{self, Write};
use std::sync::Mutex;

async fn init_binance() -> OpenLimitsWs<BinanceWebsocket> {
    OpenLimitsWs {
        websocket: BinanceWebsocket::new(BinanceParameters::prod())
            .await
            .expect("Failed to create Client"),
    }
}

async fn init_coinbase() -> OpenLimitsWs<CoinbaseWebsocket> {
    OpenLimitsWs {
        websocket: CoinbaseWebsocket::new(CoinbaseParameters::prod()),
    }
}

struct Data {
    asks: Option<Vec<AskBid>>,
    bids: Option<Vec<AskBid>>,
}

lazy_static! {
    static ref DATA_A: Mutex<Data> = Mutex::new(Data {
        bids: None,
        asks: None
    });
    static ref DATA_B: Mutex<Data> = Mutex::new(Data {
        bids: None,
        asks: None
    });
}

fn arbitrage(ex: &str, asks: Vec<AskBid>, bids: Vec<AskBid>) {
    let mut state_a = DATA_A.lock().expect("Failed to get a lock on state_a");
    let mut state_b = DATA_B.lock().expect("Failed to get a lock on state_b");
    match ex {
        "a" => {
            *state_a = Data {
                asks: Some(asks),
                bids: Some(bids),
            }
        }
        "b" => {
            *state_b = Data {
                asks: Some(asks),
                bids: Some(bids),
            }
        }
        _ => println!("Unknown exchange"),
    }

    if let (Some(b), Some(a)) = (state_b.bids.clone(), state_a.asks.clone()) {
        for bid_b in &b {
            for ask_a in &a {
                if bid_b.qty == ask_a.qty
                    && bid_b.price > ask_a.price
                    && bid_b.qty > Decimal::new(0, 8)
                {
                    print!("\r");
                    io::stdout().flush().expect("Failed to flush stdout");
                    println!(
                        "Buy on A for {:.2} and Sell on B for {:.2} at the amount of {}",
                        ask_a.price,
                        bid_b.price,
                        bid_b.qty.normalize()
                    );
                    *state_a = Data {
                        asks: None,
                        bids: None,
                    };
                    *state_b = Data {
                        asks: None,
                        bids: None,
                    };
                    print!("\nLooking for arbitrage opportunities...");
                    io::stdout().flush().expect("Failed to flush stdout");
                    return;
                }
            }
        }
    }

    if let (Some(a), Some(b)) = (state_a.bids.clone(), state_b.asks.clone()) {
        for bid_a in &a {
            for ask_b in &b {
                if bid_a.qty == ask_b.qty
                    && bid_a.price > ask_b.price
                    && bid_a.qty > Decimal::new(0, 8)
                {
                    print!("\r");
                    io::stdout().flush().expect("Failed to flush stdout");
                    println!(
                        "Buy on B for {:.2} and Sell on A for {:.2} at the amount of {}",
                        ask_b.price,
                        bid_a.price,
                        bid_a.qty.normalize()
                    );
                    *state_a = Data {
                        asks: None,
                        bids: None,
                    };
                    *state_b = Data {
                        asks: None,
                        bids: None,
                    };
                    print!("\nLooking for arbitrage opportunities...");
                    io::stdout().flush().expect("Failed to flush stdout");
                    return;
                }
            }
        }
    }
}
async fn orderbook_b() {
    let client = init_binance().await;
    let sub = OrderBookUpdates("btcusdt".to_string());

    client
        .subscribe(sub, move |m| {
            let r = m.as_ref();

            if let Ok(Generic(OrderBook(value))) = r {
                arbitrage("b", value.clone().asks, value.clone().bids);
            } else if let Err(value) = r {
                println!("{:#?}", value);
            }
        })
        .await
        .expect("Failed to subscribe to orderbook on Binance");
}
async fn orderbook_c() {
    let client = init_coinbase().await;
    let sub = OrderBookUpdates("BTC-USD".to_string());

    client
        .subscribe(sub, move |m| {
            let r = m.as_ref();
            if let Ok(Generic(OrderBook(value))) | Ok(Generic(OrderBookDiff(value))) = r {
                arbitrage("a", value.clone().asks, value.clone().bids);
            } else if let Err(value) = r {
                println!("{:#?}", value);
            }
        })
        .await
        .expect("Failed to subscribe to orderbook on Coinbase");
}

#[tokio::main]
async fn main() {
    println!("Welcome! Arbitrage opportunities will show up as they happen.\n");
    orderbook_b().await;
    orderbook_c().await;
    loop {}
}
