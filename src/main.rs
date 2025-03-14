mod sequencer;
mod display;
mod controller;

use crossterm::{                                                                                                                              
    event::{self, Event as CEvent, KeyCode},                                                                                                  
    execute,                                                                                                                                  
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen}                                                                   
};                                                                                                                                            
use rodio::OutputStream;                                                                                     
use std::{io, sync::mpsc, thread, time::Duration};
use std::sync::Arc;
use std::thread::yield_now;
                                                                                                                                             
fn main() -> Result<(), Box<dyn std::error::Error>> {      
    let pwd = env!("CARGO_MANIFEST_DIR");       
    println!("{}", pwd);                                                                             
    // Set up the audio output                                                                                                                
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let stream_handle = Arc::new(stream_handle);                                                                                                                                                                                          
                                                                                                                                              
    // Setup terminal                                                                                                                         
    let mut stdout = io::stdout();                                                                                                            
    execute!(stdout, EnterAlternateScreen)?;                                                                                                  
    terminal::enable_raw_mode()?;                                                                                                             
                                                                                                                                              
    // Event handling                                                                                                                         
    let (tx, rx) = mpsc::channel();                                                                                                           
    thread::spawn(move || {                                                                                                                   
        loop {                                                                                                                                
            if event::poll(Duration::from_millis(5)).unwrap() {                                                                             
                if let CEvent::Key(key_event) = event::read().unwrap() {                                                                      
                    tx.send(key_event).unwrap();                                                                                              
                }                                                                                                                             
            }                                                                                                                                 
        }                                                                                                                                     
    });      

    let mut seq = sequencer::Sequencer::new(8, stream_handle);

    let seq_state_rx = seq.get_state_rx();
    let mut ctrl = controller::Controller::new(seq_state_rx, display::CLIDisplay::new()?);
    thread::spawn(move || {
        ctrl.run_loop();
    });

    seq.set_tempo(160);
    seq.set_division(sequencer::Division::S);

    let sample_hat = sequencer::BufferedSample::load_from_file(&format!("{pwd}/one_shots/hat0.wav").to_string())?;
    let sample_hat = Arc::new(sample_hat);
    let trk_hat = seq.add_track("Hat".to_string(), Arc::clone(&sample_hat))?;
    trk_hat.set_slots_vel(&[32, 127, 32, 108, 32, 127, 32, 108]);

    let sample_kick = sequencer::BufferedSample::load_from_file(&format!("{pwd}/one_shots/kick0.wav").to_string())?;
    let sample_kick = Arc::new(sample_kick);
    let trk_kick = seq.add_track("Kick".to_string(), Arc::clone(&sample_kick))?;
    trk_kick.set_slots_vel(&[127, 0, 56, 127, 0, 127, 0, 75]);

    let sample_snare = sequencer::BufferedSample::load_from_file(&format!("{pwd}/one_shots/snare0.wav").to_string())?;
    let sample_snare = Arc::new(sample_snare);
    let trk_snare = seq.add_track("Snare".to_string(), Arc::clone(&sample_snare))?;
    trk_snare.set_slots_vel(&[0, 0, 0, 127, 0, 47, 0, 127]);

    thread::spawn(move || {
        seq.run_loop();
    });                                                                                                                        
                                                                                                                                              
    loop {                                                                                                                                    
        if let event::KeyEvent {                                                                                                                 
                code: KeyCode::Esc,                                                                                                           
                ..                                                                                                                         
            } = rx.recv()? {                                                                                                                            
            
            break;
        }
        yield_now();                                                                                                                 
    }                                                                                                                              
                                                                                                                                              
    // Cleanup                                                                                                                                
    terminal::disable_raw_mode()?;                                                                                                            
    execute!(stdout, LeaveAlternateScreen)?;                                                                                                  
    Ok(())                                                                                                                                    
} 