use crate::sequencer::{State, Command, TrackState, Division};
use prost::Message;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::convert::TryFrom;
use zmq;

// Include the generated Protocol Buffer code
include!(concat!(env!("OUT_DIR"), "/rdum.rs"));

/// Serializes a sequencer::State into a Protocol Buffers message
pub fn serialize_state(state: &State) -> Result<Vec<u8>, Box<dyn Error>> {
    // Convert the Rust State to the Protocol Buffer State
    let proto_state = rdum::State {
        tempo: state.tempo as u32,
        trk_idx: state.trk_idx as u64,
        trks: state.trks.iter().map(|track| rdum::TrackState {
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
fn command_to_proto_message(cmd: &Command) -> rdum::CommandMessage {
    let mut message = rdum::CommandMessage {
        command_type: match cmd {
            Command::Play => rdum::Command::CommandPlay,
            Command::Stop => rdum::Command::CommandStop,
            Command::Reset => rdum::Command::CommandReset,
            Command::SetTempo(_) => rdum::Command::CommandSetTempo,
            Command::SetDivision(_) => rdum::Command::CommandSetDivision,
            Command::SetTrack(_) => rdum::Command::CommandSetTrack,
            Command::ToggleSlot(_) => rdum::Command::CommandToggleSlot,
            Command::ClearTrack => rdum::Command::CommandClearTrack,
            Command::AddTrack(_) => rdum::Command::CommandAddTrack,
            Command::DeleteTrack(_) => rdum::Command::CommandDeleteTrack,
            Command::RenameTrack(_) => rdum::Command::CommandRenameTrack,
            _ => rdum::Command::CommandUnspecified,
        } as i32,
        args: None,
    };
    
    // Set the appropriate argument based on the command type
    match cmd {
        Command::SetTempo(tempo) => {
            message.args = Some(rdum::command_message::Args::Tempo(*tempo as u32));
        },
        Command::SetDivision(division) => {
            message.args = Some(rdum::command_message::Args::Division(*division as u32));
        },
        Command::SetTrack(track_idx) => {
            message.args = Some(rdum::command_message::Args::TrackIndex(*track_idx as u64));
        },
        Command::ToggleSlot(slot_idx) => {
            message.args = Some(rdum::command_message::Args::ToggleSlot(rdum::ToggleSlotArgs {
                slot_index: *slot_idx as u64,
            }));
        },
        Command::AddTrack(name) => {
            message.args = Some(rdum::command_message::Args::NewTrackName(name.clone()));
        },
        Command::DeleteTrack(track_idx) => {
            message.args = Some(rdum::command_message::Args::DeleteTrackIndex(*track_idx as u64));
        },
        Command::RenameTrack((track_idx, name)) => {
            message.args = Some(rdum::command_message::Args::RenameTrack(rdum::RenameTrackArgs {
                track_index: *track_idx as u64,
                new_name: name.clone(),
            }));
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
    let msg = socket.recv_msg(0)?;
    let command_msg = rdum::CommandMessage::decode(msg)?;
    
    // Convert the Protocol Buffer Command to the Rust Command
    proto_message_to_command(&command_msg)
}

/// Helper function to convert a Protocol Buffer CommandMessage to Rust Command
fn proto_message_to_command(proto_cmd: &rdum::CommandMessage) -> Result<Command, Box<dyn Error>> {
    // Convert the command type
    let cmd_type = match rdum::Command::try_from(proto_cmd.command_type) {
        Ok(cmd) => cmd,
        Err(_) => return Err("Invalid command type".into()),
    };
    
    // Handle the command arguments
    let cmd = match cmd_type {
        rdum::Command::CommandPlay => Command::Play,
        rdum::Command::CommandStop => Command::Stop,
        rdum::Command::CommandReset => Command::Reset,
        rdum::Command::CommandSetTempo => {
            if let Some(rdum::command_message::Args::Tempo(tempo)) = &proto_cmd.args {
                Command::SetTempo(*tempo as u8)
            } else {
                return Err("Missing tempo argument for SetTempo command".into());
            }
        },
        rdum::Command::CommandSetDivision => {
            if let Some(rdum::command_message::Args::Division(div_value)) = &proto_cmd.args {
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
        rdum::Command::CommandSetTrack => {
            if let Some(rdum::command_message::Args::TrackIndex(track_idx)) = &proto_cmd.args {
                Command::SetTrack(*track_idx as usize)
            } else {
                return Err("Missing track index argument for SetTrack command".into());
            }
        },
        rdum::Command::CommandToggleSlot => {
            if let Some(rdum::command_message::Args::ToggleSlot(toggle_slot)) = &proto_cmd.args {
                Command::ToggleSlot(toggle_slot.slot_index as usize)
            } else {
                return Err("Missing slot index argument for ToggleSlot command".into());
            }
        },
        rdum::Command::CommandClearTrack => Command::ClearTrack,
        rdum::Command::CommandAddTrack => {
            if let Some(rdum::command_message::Args::NewTrackName(name)) = &proto_cmd.args {
                Command::AddTrack(name.clone())
            } else {
                return Err("Missing name argument for AddTrack command".into());
            }
        },
        rdum::Command::CommandDeleteTrack => {
            if let Some(rdum::command_message::Args::DeleteTrackIndex(track_idx)) = &proto_cmd.args {
                Command::DeleteTrack(*track_idx as usize)
            } else {
                return Err("Missing track index argument for DeleteTrack command".into());
            }
        },
        rdum::Command::CommandRenameTrack => {
            if let Some(rdum::command_message::Args::RenameTrack(rename_track)) = &proto_cmd.args {
                Command::RenameTrack((rename_track.track_index as usize, rename_track.new_name.clone()))
            } else {
                return Err("Missing arguments for RenameTrack command".into());
            }
        },
        _ => return Err("Unsupported command type".into()),
    };
    
    Ok(cmd)
}