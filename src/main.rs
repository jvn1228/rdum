mod sequencer;
mod controller;

use ratatui;                                                                                           
use rodio::OutputStream;                                                                                     
use std::{thread, time::Duration, io};
use std::sync::Arc;
use std::error::Error;
use sequencer::Command;
use controller::cli::CLIController;
use crossterm::{event::{self, Event, KeyCode}, terminal};
use midir::MidiOutput;

use sequencer::ChokeGrp;
                                                                                                                                             
fn main() -> Result<(), Box<dyn Error>> {      
    let pwd = env!("CARGO_MANIFEST_DIR");       
    println!("{}", pwd);                                                                             
    // Set up the audio output                                                                                                                
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let stream_handle = Arc::new(stream_handle);                                                                                                                                                                                             

    let mut seq = sequencer::Sequencer::new(stream_handle);

    let midi_out = MidiOutput::new("Sequencer")?;
    for port in midi_out.ports() {
        println!("{}", port.id());
    }
    let port = midi_out.find_port_by_id("16:0".to_string()).unwrap();
    seq.connect_midi(port).unwrap();

    let seq_state_rx = seq.get_state_rx();
    let seq_cmd_tx = seq.get_command_tx();
    let mut ctrl = CLIController::new(seq_state_rx, seq_cmd_tx);

    // seq.set_tempo(90);
    seq.set_division(sequencer::Division::E);

    let trk_hat = seq.add_track("kit0/hat.wav".to_string())?;
    trk_hat.set_slots_vel(&[50, 0, 0, 0, 0, 127, 32, 0]);

    let trk_kick = seq.add_track("kit0/kick.wav".to_string())?;
    trk_kick.set_slots_vel(&[127, 0, 0, 90, 127, 0, 0, 75]);

    let trk_snare = seq.add_track("kit0/snare.wav".to_string())?;
    trk_snare.set_slots_vel(&[0, 0, 127, 0, 0, 47, 127, 0]);         

    let trk_open_hat = seq.add_track("kit0/open_hat.wav".to_string())?;
    trk_open_hat.set_slots_vel(&[0, 0, 0, 0, 0, 0, 0, 127]);         

    let seq_ctx_handle = seq.ctx.clone();

    seq_ctx_handle.with_lock(|props| {
        props.patterns[0].choke_grps.push(ChokeGrp::new(vec![0, 3]));
    });

    let mut web_ctrl = controller::web::WebController::new(seq.get_command_tx(), seq.get_state_rx());
    thread::spawn(move || {
        web_ctrl.run();
    });
    let mut zmq_ctrl = controller::zeromq::ZeroMQController::new(seq.get_command_tx(), seq.get_state_rx());
    thread::spawn(move || {
        zmq_ctrl.run();
    });

    seq.play();
    // thread::spawn(move || {
    //     sequencer::Sequencer::run_sound_loop(seq);
    // });
    thread::spawn(move || {
        sequencer::Sequencer::run_command_loop(seq_ctx_handle);
    });                                                                                            
                                                                                                                           
    // let mut terminal = ratatui::init();
    // let app_result = ctrl.run(&mut terminal);
    // ratatui::restore();
    // app_result?;
    // Configure terminal for non-blocking input
    terminal::enable_raw_mode().expect("Failed to enable raw mode");
    
    // println!("Running (press 'q' to exit)...");
    
    // Main loop with key detection
    loop {
        // Check for keypress events without blocking
        if event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(key_event) = event::read().unwrap() {
                if key_event.code == KeyCode::Char('q') {
                    println!("\nReceived 'q' key press. Shutting down...");
                    break;
                }
            }
        }
        
        seq.play_next();
        seq.sleep();

        thread::yield_now();
    }
    
    // Clean up terminal settings
    terminal::disable_raw_mode().expect("Failed to disable raw mode");
    println!("Gracefully shutting down.");

    Ok(())
} 