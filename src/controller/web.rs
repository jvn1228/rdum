use std::sync::mpsc;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use async_tungstenite::{tokio::accept_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use crate::sequencer::{Command, State, Division};
use serde_json;
use serde;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum MessageType {
    #[serde(rename = "state_update")]
    StateUpdate,
    #[serde(rename = "play_sequencer")]
    PlaySequencer,
    #[serde(rename = "stop_sequencer")]
    StopSequencer,
    #[serde(rename = "set_tempo")]
    SetTempo,
    #[serde(rename = "set_pattern")]
    SetPattern,
    #[serde(rename = "set_division")]
    SetDivision,
    #[serde(rename = "play_sound")]
    PlaySound,
    #[serde(rename = "set_slot_velocity")]
    SetSlotVelocity,
    #[serde(rename = "set_track_length")]
    SetTrackLength,
    #[serde(rename = "add_pattern")]
    AddPattern,
    #[serde(rename = "remove_pattern")]
    RemovePattern,
    #[serde(rename = "select_pattern")]
    SelectPattern,
    #[serde(rename = "set_pattern_length")]
    SetPatternLength,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WebSocketMessage {
    #[serde(rename = "type")]
    msg_type: MessageType,
    payload: serde_json::Value,
}
pub struct WebController {
    addr: SocketAddr,
    cmd_tx_ch: mpsc::Sender<Command>,
    state_rx_ch: mpsc::Receiver<State>,
}

impl WebController {
    pub fn new(cmd_tx_ch: mpsc::Sender<Command>, state_rx_ch: mpsc::Receiver<State>) -> Self {
        Self {
            addr: "0.0.0.0:8080".parse().unwrap(),
            cmd_tx_ch,
            state_rx_ch,
        }
    }

    pub fn run(&mut self) {
        let addr = self.addr;
        // Take ownership of the receiver, we can't clone it
        let state_rx_ch = std::mem::replace(&mut self.state_rx_ch, mpsc::channel().1);
        
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
                let cmd_tx_ch = self.cmd_tx_ch.clone();
                // Send an initial message to confirm connection works
                tokio::spawn(async move {
                    // Small delay to ensure connection is fully established
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    handle_connection(stream, state_broadcaster_rx, cmd_tx_ch).await;
                });
            }
        });
    }
    
}

async fn handle_connection(stream: TcpStream, mut state_rx: broadcast::Receiver<State>, cmd_tx_ch: mpsc::Sender<Command>) {
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
                            payload: serde_json::to_value(state).unwrap(),
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
                            let message: WebSocketMessage = serde_json::from_str(&text).unwrap();
                            match message.payload.as_object() {
                                Some(payload) => {
                                    match message.msg_type {
                                        MessageType::PlaySequencer => {
                                            cmd_tx_ch.send(Command::PlaySequencer).unwrap();
                                        },
                                        MessageType::StopSequencer => {
                                            cmd_tx_ch.send(Command::StopSequencer).unwrap();
                                        },
                                        MessageType::SetTempo => {
                                            let tempo = payload.get("tempo").unwrap().as_i64().unwrap() as u8;
                                            cmd_tx_ch.send(Command::SetTempo(tempo)).unwrap();
                                        },
                                        MessageType::SetPattern => {
                                            let pattern_idx = payload.get("pattern_idx").unwrap().as_i64().unwrap() as usize;
                                            cmd_tx_ch.send(Command::SetPattern(pattern_idx)).unwrap();
                                        },
                                        MessageType::SetDivision => {
                                            cmd_tx_ch.send(Command::SetDivision(Division::W)).unwrap();
                                        },
                                        MessageType::PlaySound => {
                                            let track_idx = payload.get("trackId").unwrap().as_i64().unwrap() as usize;
                                            cmd_tx_ch.send(Command::PlaySound(track_idx, 127)).unwrap();
                                        },
                                        MessageType::SetSlotVelocity => {
                                            let track_idx = payload.get("trackId").unwrap().as_i64().unwrap() as usize;
                                            let slot_idx = payload.get("slotIdx").unwrap().as_i64().unwrap() as usize;
                                            let velocity = payload.get("velocity").unwrap().as_i64().unwrap() as u8;
                                            cmd_tx_ch.send(Command::SetSlotVelocity(track_idx, slot_idx, velocity)).unwrap();
                                        },
                                        MessageType::SetTrackLength => {
                                            let track_idx = payload.get("track_idx").unwrap().as_i64().unwrap() as usize;
                                            cmd_tx_ch.send(Command::SetTrackLength(track_idx)).unwrap();
                                        },
                                        MessageType::AddPattern => {
                                            cmd_tx_ch.send(Command::AddPattern).unwrap();
                                        },
                                        MessageType::RemovePattern => {
                                            let pattern_id = payload.get("patternId").unwrap().as_i64().unwrap() as usize;
                                            cmd_tx_ch.send(Command::RemovePattern(pattern_id)).unwrap();
                                        },
                                        MessageType::SelectPattern => {
                                            let pattern_id = payload.get("patternId").unwrap().as_i64().unwrap() as usize;
                                            cmd_tx_ch.send(Command::SelectPattern(pattern_id)).unwrap();
                                        },
                                        MessageType::SetPatternLength => {
                                            let length = payload.get("length").unwrap().as_i64().unwrap() as usize;
                                            cmd_tx_ch.send(Command::SetPatternLength(length)).unwrap();
                                        },
                                        _ => {
                                            println!("[{}] Received unknown message: {}", peer, &text);
                                        }
                                    }
                                },
                                None => {
                                    println!("[{}] Received bad message payload: {}", peer, message.payload);
                                }
                            }
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