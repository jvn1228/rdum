use crate::sequencer::{State, Command, TrackState, Division};
use prost::Message;
use std::error::Error;
use std::convert::TryFrom;
use zmq;
use prost_types;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

pub mod state {
    // Include the generated Protocol Buffer code
    include!(concat!(env!("OUT_DIR"), "/rdum.state.rs"));
}

// Bring in the specific types from the protobuf module
use state::command_message;
use state::Command as ProtoCommand;

/// Serializes a sequencer::State into a Protocol Buffers message
pub fn serialize_state(state: &State) -> Result<Vec<u8>, Box<dyn Error>> {
    // Convert the Rust State to the Protocol Buffer State
    let proto_state = state::State {
        tempo: state.tempo as u32,
        trk_idx: state.trk_idx as u64,
        trks: state.trks.iter().map(|track| state::TrackState {
            slots: track.slots.iter().map(|&slot| slot as u32).collect(),
            name: track.name.clone(),
        }).collect(),
        division: state.division as u32,
        len: state.len as u64,
        latency: Some(prost_types::Duration {
            seconds: state.latency.as_secs() as i64,
            nanos: state.latency.subsec_nanos() as i32,
        }),
        playing: state.playing,
    };

    // Serialize the Protocol Buffer message
    let mut buf = Vec::new();
    proto_state.encode(&mut buf)?;
    Ok(buf)
}

/// Helper function to convert a Command enum to its Protocol Buffer representation with arguments
fn command_to_proto_message(cmd: &Command) -> state::CommandMessage {
    let mut message = state::CommandMessage {
        command_type: match cmd {
            Command::PlaySequencer => ProtoCommand::PlaySequencer,
            Command::StopSequencer => ProtoCommand::StopSequencer,
            Command::SetTempo(_) => ProtoCommand::SetTempo,
            _ => ProtoCommand::Unspecified,
        } as i32,
        args: None,
    };
    
    // Set the appropriate argument based on the command type
    match cmd {
        Command::SetTempo(tempo) => {
            message.args = Some(command_message::Args::Tempo(*tempo as u32));
        },
        Command::SetDivision(div) => {
            message.args = Some(command_message::Args::Division(div.clone() as u32));
        },
        _ => {}, // No arguments for other commands
    }
    
    message
}

/// Send the serialized state over ZeroMQ
pub fn send_state(socket: &zmq::Socket, state: &State) -> Result<(), Box<dyn Error>> {
    let serialized = serialize_state(state)?;
    socket.send(&serialized, 0)?;
    Ok(())
}

/// Receive and deserialize a command from ZeroMQ
pub fn receive_command(socket: &zmq::Socket) -> Result<Command, Box<dyn Error>> {
    let msg = socket.recv_bytes(0)?;
    let command_msg = state::CommandMessage::decode(msg.as_slice())?;
    
    // Convert the Protocol Buffer Command to the Rust Command
    proto_message_to_command(&command_msg)
}

/// Helper function to convert a Protocol Buffer CommandMessage to Rust Command
fn proto_message_to_command(proto_cmd: &state::CommandMessage) -> Result<Command, Box<dyn Error>> {
    // Convert the command type
    let cmd_type = match ProtoCommand::try_from(proto_cmd.command_type) {
        Ok(cmd) => cmd,
        Err(_) => return Err("Invalid command type".into()),
    };
    
    // Handle the command arguments
    let cmd = match cmd_type {
        ProtoCommand::PlaySequencer => Command::PlaySequencer,
        ProtoCommand::StopSequencer => Command::StopSequencer,
        ProtoCommand::SetTempo => {
            if let Some(command_message::Args::Tempo(tempo)) = &proto_cmd.args {
                Command::SetTempo(*tempo as u8)
            } else {
                return Err("Missing tempo argument for SetTempo command".into());
            }
        },
        ProtoCommand::SetDivision => {
            if let Some(command_message::Args::Division(div_value)) = &proto_cmd.args {
                // Convert the numeric division value to the Division enum
                let division = match *div_value {
                    2 => Division::H,
                    3 => Division::QD,
                    4 => Division::Q,
                    6 => Division::ED,
                    8 => Division::E,
                    12 => Division::SD,
                    16 => Division::S,
                    24 => Division::TD,
                    32 => Division::T,
                    _ => return Err(format!("Invalid division value: {}", div_value).into()),
                };
                Command::SetDivision(division)
            } else {
                return Err("Missing division argument for SetDivision command".into());
            }
        },
        _ => return Err("Unspecified command type".into()),
    };
    
    Ok(cmd)
}

pub struct ZeroMQController {
    addr: String,
    cmd_tx_ch: mpsc::Sender<Command>,
    state_rx_ch: mpsc::Receiver<State>,
    last_state: State,
}

impl ZeroMQController {
    pub fn new(cmd_tx_ch: mpsc::Sender<Command>, state_rx_ch: mpsc::Receiver<State>) -> Self {
        Self {
            addr: "tcp://*:5555".to_string(),
            cmd_tx_ch,
            state_rx_ch,
            last_state: State::default(),
        }
    }

    pub fn run(&mut self) {
        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::REP).unwrap();
        if let Err(e) = socket.bind(&self.addr) {
            eprintln!("Failed to bind socket: {}", e);
            return;
        }

        let mut polled_items = [socket.as_poll_item(zmq::POLLIN)];
        
        loop {
            if let Ok(state) = self.state_rx_ch.try_recv() {
                self.last_state = state;
            }
            
            // Poll with zero timeout for non-blocking behavior
            if zmq::poll(&mut polled_items, 0).is_ok() {
                // Check if our socket has events
                if polled_items[0].get_revents().contains(zmq::POLLIN) {
                    match socket.recv_msg(zmq::DONTWAIT) {
                        Ok(_) => {},
                        Err(e) if e == zmq::Error::EAGAIN => {}, // No message available
                        Err(_) => {},
                    }
                    match send_state(&socket, &self.last_state) {
                        Ok(_) => {},
                        Err(_) => {},
                    }
                }
            }
            thread::yield_now();
        }
    }
}