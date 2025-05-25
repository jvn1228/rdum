use std::sync::mpsc;
use std::net::SocketAddr;
use async_tungstenite::tungstenite::Utf8Bytes;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use async_tungstenite::{tokio::accept_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use crate::sequencer::{Command, State};
use serde_json;
use std::thread;
use serde;

#[derive(Debug, serde::Serialize)]
enum MessageType {
    #[serde(rename = "state_update")]
    StateUpdate}

#[derive(Debug, serde::Serialize)]
struct WebSocketMessage<T> {
    #[serde(rename = "type")]
    msg_type: MessageType,
    payload: T,
}
pub struct WebController {
    addr: SocketAddr,
    cmd_tx_ch: mpsc::Sender<Command>,
    state_rx_ch: mpsc::Receiver<State>,
}

impl WebController {
    pub fn new(cmd_tx_ch: mpsc::Sender<Command>, state_rx_ch: mpsc::Receiver<State>) -> Self {
        Self {
            addr: "127.0.0.1:8080".parse().unwrap(),
            cmd_tx_ch,
            state_rx_ch,
        }
    }

    pub fn run(&mut self) {
        let addr = self.addr;
        // Take ownership of the receiver, we can't clone it
        let state_rx_ch = std::mem::replace(&mut self.state_rx_ch, mpsc::channel().1);
        
        // Create a dedicated thread for the WebSocket server
        thread::spawn(move || {
            // Create a runtime for the async code
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async move {
                // Start the WebSocket server
                let listener = TcpListener::bind(&addr).await.unwrap();
                println!("WebSocket server listening on: {}", addr);
                
                // Create a channel for state distribution in the async context
                let (state_broadcaster_tx, _) = broadcast::channel::<State>(100);
                let state_broadcaster_tx_clone = state_broadcaster_tx.clone();
                
                // Start a task to receive states from the sync channel and broadcast them
                tokio::spawn(async move {
                    loop {
                        let state = state_rx_ch.recv();
                        match state {
                            Ok(state) => {
                                if state_broadcaster_tx_clone.receiver_count() > 0 {
                                    // Forward to all registered clients via broadcast channel
                                    if let Err(_) = state_broadcaster_tx_clone.send(state) {
                                        println!("Failed to broadcast");
                                    }
                                }
                            },
                            Err(e) => {
                                println!("State receiver error: {:?}", e);
                                break;
                            }
                        }
                        tokio::task::yield_now().await;
                    }
                });
                
                // Accept new WebSocket connections
                while let Ok((stream, _)) = listener.accept().await {
                    let peer = stream.peer_addr().unwrap();
                    println!("Connection from: {}", peer);
                    
                    let state_broadcaster_rx = state_broadcaster_tx.subscribe();
                    println!("Created new subscriber for {}", peer);
                    
                    // Send an initial message to confirm connection works
                    tokio::spawn(async move {
                        // Small delay to ensure connection is fully established
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        handle_connection(stream, state_broadcaster_rx).await;
                    });
                }
            });
        });
    }
}

async fn handle_connection(stream: TcpStream, mut state_rx: broadcast::Receiver<State>) {
    let peer = stream.peer_addr().unwrap();
    println!("Starting WebSocket handling for {}", peer);
    
    let ws_stream = accept_async(stream).await.expect("Failed to accept websocket connection");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Send an initial connection message to verify the WebSocket works
    let welcome_msg = serde_json::json!({"type": "connection", "status": "established"}).to_string();
    println!("Sending welcome message to {}: {}", peer, welcome_msg);
    if let Err(e) = ws_sender.send(Message::Text(welcome_msg.into())).await {
        println!("Failed to send welcome message: {:?}", e);
        return;
    }
    
    // Use select to handle both WebSocket messages and state broadcasts
    loop {
        tokio::select! {
            // Handle incoming state updates
            state_result = state_rx.recv() => {
                match state_result {
                    Ok(state) => {
                        let message = WebSocketMessage {
                            msg_type: MessageType::StateUpdate,
                            payload: state,
                        };
                        let message_json = serde_json::to_string(&message).unwrap();
                        if let Err(e) = ws_sender.send(Message::Text(message_json.into())).await {
                            println!("[{}] WebSocket send error: {:?}", peer, e);
                            break;
                        }
                    },
                    Err(e) => {
                        println!("[{}] Broadcast channel error: {:?}", peer, e);
                        // Don't break on lag error, just resubscribe
                        if e.to_string().contains("lagged") {
                            println!("[{}] Receiver lagged, continuing", peer);
                            continue;
                        }
                        break;
                    }
                }
            },
            
            // Handle incoming WebSocket messages
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            println!("[{}] Client sent close frame", peer);
                            break;
                        }
                        
                        // Handle any client messages here if needed
                        if let Message::Text(text) = msg {
                            println!("[{}] Received client message: {}", peer, text);
                        }
                    },
                    Some(Err(e)) => {
                        println!("[{}] WebSocket receive error: {:?}", peer, e);
                        break;
                    },
                    None => {
                        println!("[{}] WebSocket stream ended", peer);
                        break;
                    }
                }
            }
        }
    }
    
    println!("[{}] WebSocket connection closed", peer);
}