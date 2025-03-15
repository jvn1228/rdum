use crate::display;
use crate::sequencer;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Controller<T: display::Display> {
    state_rx: mpsc::Receiver<sequencer::State>,
    display: T,
    refresh_interval: Duration,
}

impl<T: display::Display> Controller<T> {
    pub fn new(state_rx: mpsc::Receiver<sequencer::State>, display: T) -> Controller<T> {
        Controller {
            state_rx,
            display,
            refresh_interval: Duration::from_secs_f32(1.0/12.0)
        }
    }

    // Still need a refresh rate and throw out in between msgs
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