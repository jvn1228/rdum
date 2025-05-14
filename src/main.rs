mod sequencer;
mod controller;

use ratatui;                                                                                           
use rodio::OutputStream;                                                                                     
use std::{thread, time::Duration};
use std::sync::Arc;
use std::error::Error;
use sequencer::Command;

fn new_buffered_sample(fp: &str) -> Result<Arc<sequencer::BufferedSample>, Box<dyn Error>> {
    let pwd = env!("CARGO_MANIFEST_DIR");  
    let sample = sequencer::BufferedSample::load_from_file(&format!("{pwd}/{fp}").to_string())?;
    Ok(Arc::new(sample))
}
                                                                                                                                             
fn main() -> Result<(), Box<dyn Error>> {      
    let pwd = env!("CARGO_MANIFEST_DIR");       
    println!("{}", pwd);                                                                             
    // Set up the audio output                                                                                                                
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let stream_handle = Arc::new(stream_handle);                                                                                                                                                                                             

    let mut seq = sequencer::Sequencer::new(8, stream_handle);

    let seq_state_rx = seq.get_state_rx();

    let mut ctrl = controller::CLIController::new(seq_state_rx);

    // seq.set_tempo(90);
    seq.set_division(sequencer::Division::E);

    let sample_hat = new_buffered_sample("one_shots/hat0.wav")?;
    let trk_hat = seq.add_track("Hat".to_string(), Arc::clone(&sample_hat))?;
    trk_hat.set_slots_vel(&[32, 127, 32, 108, 32, 127, 32, 108]);

    let sample_kick = new_buffered_sample("one_shots/kick0.wav")?;
    let trk_kick = seq.add_track("Kick".to_string(), Arc::clone(&sample_kick))?;
    trk_kick.set_slots_vel(&[127, 0, 0, 90, 127, 0, 0, 75]);

    let sample_snare = new_buffered_sample("one_shots/snare0.wav")?;
    let trk_snare = seq.add_track("Snare".to_string(), Arc::clone(&sample_snare))?;
    trk_snare.set_slots_vel(&[0, 0, 127, 0, 0, 47, 127, 0]);          

    let seq_props_handle = seq.props.clone();
    let cmd_tx_ch = seq.get_command_tx();

    thread::spawn(move || {
        sequencer::Sequencer::run_sound_loop(seq);
    });
    thread::spawn(move || {
        sequencer::Sequencer::run_command_loop(seq_props_handle);
    });

    thread::spawn(move || {
        let mut i = 0;
        let cmds = vec![Command::SetTempo(155), Command::SetTempo(45)];
        thread::sleep(Duration::from_secs(1));
        loop {
            cmd_tx_ch.send(cmds[i]).unwrap();
            i = (i+1) % 2;
            thread::sleep(Duration::from_secs(3));
        }
    });                                                                                                
                                                                                                                           
    let mut terminal = ratatui::init();
    let app_result = ctrl.run(&mut terminal);
    ratatui::restore();
    app_result?;
    Ok(())
} 