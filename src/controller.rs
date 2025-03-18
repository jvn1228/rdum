mod display;

use crate::sequencer;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use display::Display;

pub trait Controller {
    fn run_display_loop(&mut self) -> ();
    fn run_input_loop(&mut self) -> ();

}
// Soooooo we're gonna need a unique controller-input-display combo depending on the hardware
// or UI so there's no point have a common controller, maybe implement as a trait?
// A display may simply have the method draw and maybe clear? IDK
// Maybe everything should be rolled into controller, ie CLIController
pub struct CLIController {
    state_rx: mpsc::Receiver<sequencer::State>,
    display: display::CLIDisplay,
    refresh_interval: Duration,
}

impl CLIController {
    pub fn new(state_rx: mpsc::Receiver<sequencer::State>) -> Result<CLIController, Box<dyn std::error::Error>> {
        let display = display::CLIDisplay::new()?;
        Ok(CLIController {
            state_rx,
            display,
            refresh_interval: Duration::from_secs_f32(1.0/60.0)
        })
    }

    // hmm so we can refresh inputs at 60hz but the refresh of individual stats should also be customizable
    // perhaps as a "param" type which could even encapsulate input spec
    // write state would be replaced with primitives that the controller can use to display things
    // maybe display can have "blocks" kinda like a UI building lib that puts the cursor in the rigth spot
    pub fn run_loop(&mut self) {
        let mut last_refresh = Instant::now();

        for received in &self.state_rx {
            if Instant::now().duration_since(last_refresh) > self.refresh_interval {
                self.display.write_state(received).unwrap();
                last_refresh = Instant::now();
            }
        }
    }
}