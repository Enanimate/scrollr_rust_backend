use std::{collections::HashMap, future::pending, sync::{Arc, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};

use tokio::{net::TcpStream, time, sync::RwLock};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt, stream::{self, SplitSink, SplitStream, iter}};
use utils::database::PgPool;

use crate::{types::{TradeData, TradeUpdate, WebSocketState}, websocket::trade_service::get_trades};

const UPDATE_BATCH_SIZE: usize = 10;
const UPDATE_BATCH_TIMEOUT: u64 = 1000;
const UPDATE_BATCH_SIZE_DELAY: u64 = 500;

const LOG_THROTTLE_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) async fn connect(subscriptions: Vec<String>, api_key: String, _pool: Arc<PgPool>) {
    let state = Arc::new(RwLock::new(WebSocketState::new()));
    let url = format!("wss://ws.finnhub.io/?token={}", api_key);

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket client connected");

    let (writer, reader) = ws_stream.split();

    tokio::spawn(ws_send(writer, subscriptions));
    ws_read(reader, Arc::clone(&state)).await;
}

async fn ws_send(mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, subscriptions: Vec<String>) {
    let messages: Vec<Message> = subscriptions.iter().map(|s| {
        let sub_msg = format!(r#"{{"type":"subscribe","symbol":"{}"}}"#, s);

        Message::Text(sub_msg.into())
    }).collect();

    let mut stream = iter(messages).map(|m| Ok(m));
    writer.send_all(&mut stream).await.unwrap();
}

async fn ws_read(mut reader: SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>, state: Arc<RwLock<WebSocketState>>) {
    println!("Now listening for messages...");
    
    loop {
        tokio::select! {
            biased;
            _ = async { 
                let timer_exists = state.read().await.batch_timer.is_some();
                if timer_exists {
                    state.write().await.batch_timer.as_mut().unwrap().as_mut().await
                } else {
                    pending().await
                }} => {
                // Timer fired
                let mut state_w = state.write().await;
                state_w.batch_timer = None;
                
                if !state_w.is_processing_batch {
                    println!("Timer fired, processing batch.");
                    let state_clone = Arc::clone(&state);

                    drop(state_w);
                    tokio::spawn(process_batch(state_clone));
                } else {
                    println!("Timer fired, but a batch is already in process. Waiting.")
                }
            }

            Some(msg) = reader.next() => {
                match msg {
                    Ok(msg) => {
                        if msg.is_text() {
                            let trades_update: Result<TradeUpdate, serde_json::Error> = serde_json::from_str(&msg.to_string());
                            if let Ok(update) = trades_update {
                                if update.message_type == "trade" {
                                    handle_trade_update_batch(update.data, &state).await;
                                } else if update.message_type == "error" {
                                    eprintln!("Error message from websocket: {}", msg.to_string());
                                } else {
                                    eprintln!("Non-trade message: {:#?}", update)
                                }
                            } else {
                                eprintln!("Unexpected websocket message format: {}", msg.to_string());
                            }
                        } else if msg.is_close() {
                            println!("Server closed connection");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving message: {}", e);
                        break;
                    }
                }
            }

            else => {
                break;
            }
        }
    }

    println!("WebSocket read loop completed.");

    if !state.read().await.update_queue.is_empty() {
        println!("Processing final batch before exit...");
        process_batch(state).await;
    }
}

async fn handle_trade_update_batch(trades: Vec<TradeData>, state_arc: &Arc<RwLock<WebSocketState>>) {
    let mut state = state_arc.write().await;
    let mut new_trades = 0;

    for trade in trades.iter() {
        let ref_in_queue = state.update_queue.get(&trade.symbol);

        if let Some(trade_in_queue) = ref_in_queue {
            if trade_in_queue.timestamp >= trade.timestamp {
                continue;
            }
        }

        state.update_queue.insert(trade.symbol.clone(), trade.clone());
        new_trades += 1;
    }

    if new_trades > 0 {
        drop(state);
        schedule_batch_processing(state_arc).await;
    }
}

async fn schedule_batch_processing(state_arc: &Arc<RwLock<WebSocketState>>) {
    let mut state = state_arc.write().await;

    let delay_ms = if state.update_queue.len() >= UPDATE_BATCH_SIZE {
        UPDATE_BATCH_SIZE_DELAY
    } else {
        UPDATE_BATCH_TIMEOUT
    };
    
    let new_delay = Duration::from_millis(delay_ms);

    if let Some(timer) = &mut state.batch_timer {
        timer.as_mut().reset(time::Instant::now() + new_delay);
    } else {
        println!(
            "Scheduling batch processing in {}ms (queue: {})",
            delay_ms,
            state.update_queue.len()
        );
        state.batch_timer = Some(Box::pin(time::sleep(new_delay)));
    }
}

async fn process_batch(state_arc: Arc<RwLock<WebSocketState>>) {
    let (trades, batch_num) = {
        let mut state = state_arc.write().await;

        if state.is_processing_batch || state.update_queue.is_empty() {
            println!("Skipping batch processing (processing: {}, queue: {})", state.is_processing_batch, state.update_queue.len());
            return;
        }

        state.is_processing_batch = true;

        let trades: Vec<TradeData> = state.update_queue.values().cloned().collect();
        state.update_queue.clear();

        state.stats.batches_processed += 1;
        let batch_num = state.stats.batches_processed;

        println!("Processing batch #{} with {} trades", batch_num, trades.len());

        (trades, batch_num)
    };

    let processed_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));
    let batch_result: Result<(), anyhow::Error> = async {
        let all_trades = get_trades().await?;
        let trades_map = Arc::new(
            all_trades.into_iter().map(|t| (t.symbol.clone(), t)).collect::<HashMap<_, _>>()
        );

        let batch_size = 5;

        stream::iter(trades)
            .for_each_concurrent(batch_size, |trade| {
                let trades_map_clone = Arc::clone(&trades_map);
                let proc_clone = Arc::clone(&processed_count);
                let err_clone = Arc::clone(&error_count);

                async move {
                    match process_single_trade(trade, trades_map_clone).await {
                        Ok(_) => {
                            proc_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(e) => {
                            err_clone.fetch_add(1, Ordering::SeqCst);
                            eprintln!("Error processing trade: {}", e);
                        }
                    }
                }
            }
        ).await;

        Ok(())
    }.await;

    let mut state = state_arc.write().await;
    state.is_processing_batch = false;

    let processed = processed_count.load(Ordering::SeqCst);
    let errors = error_count.load(Ordering::SeqCst);

    match batch_result {
        Ok(_) => {
            state.stats.total_updates_processed += processed;

            let now = Instant::now();
            let should_log = state.last_log_time.map_or(true, |last| {
                now.duration_since(last) >= LOG_THROTTLE_INTERVAL
            });

            if should_log {
                state.last_log_time = Some(now);
                println!("Batch #{} complete: {} processed, {} errors",
                    batch_num, processed, errors
                );
                println!("Total updates processed: {}", state.stats.total_updates_processed);
            }
        }
        Err(e) => {
            eprintln!("Batch #{} processing error: {}", batch_num, e);
            state.stats.errors += 1;
        }
    }

    if !state.update_queue.is_empty() {
        println!("More trades queued ({}), scheduling next batch", state.update_queue.len());
        drop(state);
        schedule_batch_processing(&state_arc).await;
    }
}

async fn process_single_trade(trade: TradeData, trades_map: Arc<HashMap<String, TradeData>>) -> Result<(), anyhow::Error> {
    if let Some(db_trade) = trades_map.get(&trade.symbol) {
        println!("Processing {}: new price ${}, db_price ${}", trade.symbol, trade.price, db_trade.price);
    } else {
        println!("Processing {}: new symbol, price ${}", trade.symbol, trade.price);
    }

    tokio::time::sleep(Duration::from_millis(10)).await;

    Ok(())
}

pub mod trade_service {
    use std::time::Duration;

    use super::TradeData;
    use anyhow::Result;

    pub async fn get_trades() -> Result<Vec<TradeData>, anyhow::Error> {
        println!("Simulating database call 'get_trades()'...");
        tokio::time::sleep(Duration::from_millis(150)).await;

        let mock_data = vec![
            TradeData { symbol: "AAPL".to_string(), price: 150.0, timestamp: 0 },
            TradeData { symbol: "MSFT".to_string(), price: 300.0, timestamp: 0 },
        ];

        println!("Database call complete.");
        Ok(mock_data)
    }
}