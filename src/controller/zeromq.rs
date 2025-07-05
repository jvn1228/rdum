use crate::sequencer::{SeqState, Command, Division, Swing, StateUpdate};
use prost::Message;
use std::error::Error;
use std::convert::TryFrom;
use zmq;
use prost_types;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

pub mod state {
    // Include the generated Protocol Buffer code
    include!(concat!(env!("OUT_DIR"), "/rdum.state.rs"));
}

// Bring in the specific types from the protobuf module
use state::command_message;
use state::Command as ProtoCommand;

/// Serializes a sequencer::SeqState into a Protocol Buffers message
pub fn serialize_state(state: &SeqState) -> Result<Vec<u8>, Box<dyn Error>> {
    // Convert the Rust State to the Protocol Buffer State
    let proto_state = state::State {
        tempo: state.tempo as u32,
        trks: state.trks.iter().map(|track| state::TrackState {
            slots: track.slots.iter().map(|&slot| slot as u32).collect(),
            name: track.name.clone(),
            idx: track.idx as u64,
            len: track.len as u64,
            sample_path: track.sample_path.clone(),
        }).collect(),
        division: state.division as u32,
        default_len: state.default_len as u64,
        latency: Some(prost_types::Duration {
            seconds: state.latency.as_secs() as i64,
            nanos: state.latency.subsec_nanos() as i32,
        }),
        playing: state.playing,
        pattern_id: state.pattern_id as u64,
        pattern_len: state.pattern_len as u64,
        pattern_name: state.pattern_name.clone(),
        queued_pattern_id: state.queued_pattern_id as u64,
        swing: state.swing as u32,
    };

    // Serialize the Protocol Buffer message
    let mut buf = Vec::new();
    proto_state.encode(&mut buf)?;
    Ok(buf)
}

/// Send the serialized state over ZeroMQ
pub fn send_state(socket: &zmq::Socket, state: &SeqState) -> Result<(), Box<dyn Error>> {
    let serialized = serialize_state(state)?;
    socket.send(&serialized, 0)?;
    Ok(())
}

/// Decode a Protocol Buffer CommandMessage into a Rust Command
pub fn decode_command(msg: &[u8]) -> Result<Command, Box<dyn Error>> {
    let command_msg = state::CommandMessage::decode(msg)?;
    
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
                Command::SetDivision(Division::from(*div_value))
            } else {
                return Err("Missing division argument for SetDivision command".into());
            }
        },
        ProtoCommand::PlaySound => {
            if let Some(command_message::Args::PlaySoundArgs(play_sound_args)) = &proto_cmd.args {
                Command::PlaySound(play_sound_args.track_index as usize, play_sound_args.velocity as u8)
            } else {
                return Err("Missing arguments for PlaySound command".into());
            }
        },
        ProtoCommand::SetSlotVelocity => {
            if let Some(command_message::Args::SlotArgs(slot_args)) = &proto_cmd.args {
                Command::SetSlotVelocity(slot_args.track_index as usize, slot_args.slot_index as usize, slot_args.velocity as u8)
            } else {
                return Err("Missing arguments for SetSlotVelocity command".into());
            }
        },
        ProtoCommand::SetTrackLength => {
            if let Some(command_message::Args::TrackLengthArgs(track_length_args)) = &proto_cmd.args {
                Command::SetTrackLength(track_length_args.track_index as usize)
            } else {
                return Err("Missing arguments for SetTrackLength command".into());
            }
        },
        ProtoCommand::AddPattern => Command::AddPattern,
        ProtoCommand::RemovePattern => {
            if let Some(command_message::Args::PatternIndex(pattern_index)) = &proto_cmd.args {
                Command::RemovePattern(*pattern_index as usize)
            } else {
                return Err("Missing arguments for RemovePattern command".into());
            }
        },
        ProtoCommand::SelectPattern => {
            if let Some(command_message::Args::PatternIndex(pattern_index)) = &proto_cmd.args {
                Command::SelectPattern(*pattern_index as usize)
            } else {
                return Err("Missing arguments for SelectPattern command".into());
            }
        },
        ProtoCommand::SetPatternLength => {
            if let Some(command_message::Args::PatternLength(pattern_length)) = &proto_cmd.args {
                Command::SetPatternLength(*pattern_length as usize)
            } else {
                return Err("Missing arguments for SetPatternLength command".into());
            }
        },
        ProtoCommand::SavePattern => Command::SavePattern,
        ProtoCommand::LoadPattern => {
            if let Some(command_message::Args::PatternFname(pattern_fname)) = &proto_cmd.args {
                Command::LoadPattern(pattern_fname.clone())
            } else {
                return Err("Missing arguments for LoadPattern command".into());
            }
        },
        ProtoCommand::ListPatterns => Command::ListPatterns,
        ProtoCommand::ListSamples => Command::ListSamples,
        ProtoCommand::SetSwing => {
            if let Some(command_message::Args::Swing(swing)) = &proto_cmd.args {
                Command::SetSwing(Swing::from(*swing))
            } else {
                return Err("Missing swing argument for SetSwing command".into());
            }
        },
        ProtoCommand::AddTrack => Command::AddTrack,
        ProtoCommand::SetTrackSample => {
            if let Some(command_message::Args::TrackSampleArgs(track_sample_args)) = &proto_cmd.args {
                Command::SetTrackSample(track_sample_args.track_index as usize, track_sample_args.sample_path.clone())
            } else {
                return Err("Missing arguments for SetTrackSample command".into());
            }
        },
        _ => return Err("Unspecified command type".into()),
    };
    
    Ok(cmd)
}

pub struct ZeroMQController {
    addr: String,
    cmd_tx_ch: mpsc::Sender<Command>,
    state_rx_ch: mpsc::Receiver<StateUpdate>,
    last_state: SeqState,
}

impl ZeroMQController {
    pub fn new(cmd_tx_ch: mpsc::Sender<Command>, state_rx_ch: mpsc::Receiver<StateUpdate>) -> Self {
        Self {
            addr: "tcp://*:5555".to_string(),
            cmd_tx_ch,
            state_rx_ch,
            last_state: SeqState::default(),
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
                match state {
                    StateUpdate::SeqState(state) => self.last_state = state,
                    _ => {}
                }
            }
            
            // Poll with zero timeout for non-blocking behavior
            if zmq::poll(&mut polled_items, 0).is_ok() {
                // Check if our socket has events
                if polled_items[0].get_revents().contains(zmq::POLLIN) {
                    match socket.recv_bytes(zmq::DONTWAIT) {
                        Ok(msg) => {
                            if let Ok(command) = decode_command(&msg) {
                                self.cmd_tx_ch.send(command).unwrap();
                            }
                        },
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