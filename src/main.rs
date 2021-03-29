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
use std::sync::Mutex;
use std::io::{self, Write};

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
    let mut state_a = DATA_A.lock().unwrap();
    let mut state_b = DATA_B.lock().unwrap();
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
                    //     let iter = a
                    //       .iter()
                    //       .find(|v| v.qty == bid_b.qty && bid_b.price > v.price);
                    // if iter != None {
                    print!("\r");
                    io::stdout().flush().unwrap();
                    println!(
                        "Buy on A for {:.2} and Sell on B for {:.2} at the amount of {}",
                        //iter.unwrap().price,
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
                   io::stdout().flush().unwrap();
                    return;
                    //}
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
                    //  let iter = b
                    //        .iter()
                    //        .find(|v| v.qty == bid_a.qty && bid_a.price > v.price);
                    //     if iter != None {
                    print!("\r");
                   io::stdout().flush().unwrap();
                    println!(
                        "Buy on B for {:.2} and Sell on A for {:.2} at the amount of {}",
                        //iter.unwrap().price,
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
                   io::stdout().flush().unwrap();
                    return;
                    //    }
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
        .unwrap();
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
        .unwrap();
}

#[tokio::main]
async fn main() {
    orderbook_b().await;
    orderbook_c().await;
    loop {}
}
