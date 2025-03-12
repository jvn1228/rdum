use crossterm::{                                                                                                                              
    event::{self, Event as CEvent, KeyCode},                                                                                                  
    execute,                                                                                                                                  
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},                                                                             
};                                                                                                                                            
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};                                                                                     
use std::{io, sync::mpsc, thread, time::Duration};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::time::Instant;
use spin_sleep;
use std::io::Write;

enum Commands {
    SetTempo(f32),
    SetSlotVelocity(u8, u8),
    SetSequencerLength(usize),
    PlaySound(u8),
}

enum Division {
    W = 1,
    H = 2,
    QD = 3,
    Q = 4,
    ED = 6,
    E = 8,
    SD = 12,
    S = 16,
    TD = 24,
    T = 32,
}

pub trait Display {
    fn write_state(&self, s: SeqState) -> Result<(), Box<dyn Error>>;
}

pub struct MockDisplay {}

impl Display for MockDisplay {
    fn write_state(&self, s: SeqState) -> Result<(), Box<dyn Error>> {
        print!("\r{:?}", s);
        std::io::stdout().flush();
        Ok(())    
    }
}

#[derive(Debug, Clone)]
pub struct TrackState {
    slots: Vec<u8>,
    name: String
}

#[derive(Debug)]
pub struct SeqState {
    tempo: u8,
    trk_idx: usize,
    trks: Vec<TrackState>,
    division: u8,
    len: usize,
    latency: Duration
}

pub struct Controller<T: Display> {
    state_rx: mpsc::Receiver<SeqState>,
    display: T,
}

impl<T: Display> Controller<T> {
    pub fn new(state_rx: mpsc::Receiver<SeqState>, display: T) -> Controller<T> {
        Controller {
            state_rx,
            display
        }
    }

    // Still need a refresh rate and throw out in between msgs
    pub fn run_loop(&self) {
        for received in &self.state_rx {
            self.display.write_state(received).unwrap();
        }
    }
}

#[derive(Clone)]
pub struct BufferedSample {
    sample_rate: u32,
    channels: u16,
    current_sample: usize,
    buffer: Box<Vec<f32>>,
}

impl BufferedSample {
    pub fn load_from_file(fp: &str) -> Result<BufferedSample, Box<dyn Error>> {
        let file = File::open(fp)?;
        let decoder = rodio::Decoder::new(file)?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let decoder = decoder.convert_samples::<f32>();
        let mut buffer = Box::new(vec![]);
        for d in decoder.buffered() {
            buffer.push(d);
        }
        Ok(BufferedSample {
            sample_rate,
            channels,
            current_sample: 0,
            buffer,
        })
    }
}

impl Iterator for BufferedSample
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.current_sample >= self.buffer.len() {
            return None
        }
        let b = self.buffer[self.current_sample];
        self.current_sample = self.current_sample + 1;
        Some(b)
    }
}

impl Source for BufferedSample {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(self.buffer.len() as u64 / self.channels as u64 * 1000 / self.sample_rate as u64))
    }
}

pub struct Slot {
    pub velocity: Arc<Mutex<u8>>,
}

pub struct Track {
    pub slots: Vec<Slot>,
    pub sample: Arc<BufferedSample>,
    pub sink: Arc<Sink>,
    pub name: String,
}

impl Track {
    pub fn new(name: String, len: usize, sample: Arc<BufferedSample>, sink: Arc<Sink>) -> Track {
        let mut slots = vec![];
        for _ in 0..len {
            slots.push(Slot {
                velocity: Arc::new(Mutex::new(0))
            });
        }
        Track {
            slots,
            sample,
            sink,
            name
        }
    }

    pub fn set_slot_vel(&self, slot: usize, vel: u8) {
        *self.slots[slot].velocity.lock().unwrap() = vel;
    }

    pub fn set_slots_vel(&self, vels: &[u8]) {
        for (i, v) in vels.iter().enumerate() {
            if i >= self.slots.len() {
                break;
            }
            *self.slots[i].velocity.lock().unwrap() = *v;
        }
    }
}

pub struct Sequencer {
    pub stream: Arc<OutputStreamHandle>,
    pub tracks: Vec<Track>,
    pub trk_idx: usize,
    pub len: usize,
    // Average of current and last cycle time
    latency: Duration,
    tempo: u8,
    division: u8,
    pulse_interval: Duration,
    sleep_interval: Duration,
    // pulses per beat
    ppb: u8,
    pulse_idx: u8,
    state_ch: Vec<mpsc::Sender<SeqState>>
}

// Maybe tracks should have independent lengths?
// But we actually need to trigger notes based on 24 pulses in a second not every sleep cycle....
impl Sequencer {
    pub fn new(len: usize, stream: Arc<OutputStreamHandle>) -> Sequencer {
        Sequencer {
            stream,
            tracks: vec![],
            trk_idx: 0,
            len,
            tempo: 120,
            // allowable set{1,2,3,4,6,8,12,16,24,32}
            division: 8,
            latency: Duration::ZERO,
            pulse_interval: Duration::from_secs_f32(1.0/24.0),
            sleep_interval: Duration::from_secs_f32(1.0/24.0),
            // pulses per bar, 24 per quarter note
            ppb: 24*4,
            pulse_idx: 0,
            state_ch: vec![]
        }
    }

    pub fn set_tempo(&mut self, bpm: u8) {
        self.tempo = bpm;
    }

    pub fn add_track(&mut self, name: String, sample: Arc<BufferedSample>) -> Result<&Track, Box<dyn Error>> {
        let sink = Sink::try_new(&self.stream)?;
        let sink = Arc::new(sink);
        sink.play();
        self.tracks.push(Track::new(name, self.len, sample, sink));
        Ok(&self.tracks.last().unwrap())
    }

    pub fn play_next(&mut self) {
        let start = Instant::now();
        // hmm might have to create a spare vec of pulses where 1 is trigger to handle swing patterns
        // and then in fact we might have to move that tracking to the track
        if self.pulse_idx % (self.ppb / self.division) == 0 {
            for t in &self.tracks {
                let vel = t.slots[self.trk_idx].velocity.lock().unwrap();
                if *vel > 0 {
                    t.sink.append((*t.sample).clone().amplify(*vel as f32 / 127.0));
                    if t.sink.len() > 1 {
                        t.sink.skip_one();
                    }
                }
            }
            self.trk_idx = (self.trk_idx + 1) % self.len;
        }
        self.pulse_idx = (self.pulse_idx + 1) % self.ppb;
        self.tx_state();
        
        // to do send midi clk msg
        self.set_latency(Instant::now().duration_since(start));
    }

    pub fn latency(&self) -> Duration {
        self.latency
    }

    fn set_latency(&mut self, t: Duration) {
        self.latency = Duration::from_nanos(((self.latency + t).as_nanos() / 2) as u64);
        self.sleep_interval = self.pulse_interval - self.pulse_interval.min(self.latency);
    }

    fn set_division(&mut self, division: Division) {
        self.division = division as u8;
    }

    pub fn get_state_rx(&mut self) -> mpsc::Receiver<SeqState> {
        let (tx, rx) = mpsc::channel();
        self.state_ch.push(tx);
        rx
    }

    fn tx_state(&self) {
        let mut trks = vec![];
        for t in &self.tracks {
            trks.push(TrackState {
                slots: t.slots.iter().map(|s| { *s.velocity.lock().unwrap() }).collect(),
                name: t.name.clone()
            })
        }
        for tx in &self.state_ch {
            let _ = tx.send(SeqState {
                tempo: self.tempo,
                trk_idx: self.trk_idx,
                trks: trks.clone(),
                division: self.division,
                len: self.len,
                latency: self.latency
            });
        }
    }

    pub fn sleep(&self) {
        spin_sleep::sleep(self.sleep_interval);
    }
}

fn run_loop(sequencer: &mut Sequencer) {
    loop {
        sequencer.play_next();
        sequencer.sleep();
    }
}
                                                                                                                                              
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

    let mut sequencer = Sequencer::new(8, stream_handle);

    let seq_state_rx = sequencer.get_state_rx();
    let controller = Controller::new(seq_state_rx, MockDisplay{});
    thread::spawn(move || {
        controller.run_loop();
    });

    sequencer.set_tempo(160);
    sequencer.set_division(Division::S);

    let sample_hat = BufferedSample::load_from_file(&format!("{pwd}/one_shots/hat0.wav").to_string())?;
    let sample_hat = Arc::new(sample_hat);
    let trk_hat = sequencer.add_track("Hat".to_string(), Arc::clone(&sample_hat))?;
    trk_hat.set_slots_vel(&[32, 127, 32, 108, 32, 127, 32, 108]);

    let sample_kick = BufferedSample::load_from_file(&format!("{pwd}/one_shots/kick0.wav").to_string())?;
    let sample_kick = Arc::new(sample_kick);
    let trk_kick = sequencer.add_track("Kick".to_string(), Arc::clone(&sample_kick))?;
    trk_kick.set_slots_vel(&[127, 0, 56, 127, 0, 127, 0, 75]);

    let sample_snare = BufferedSample::load_from_file(&format!("{pwd}/one_shots/snare0.wav").to_string())?;
    let sample_snare = Arc::new(sample_snare);
    let trk_snare = sequencer.add_track("Snare".to_string(), Arc::clone(&sample_snare))?;
    trk_snare.set_slots_vel(&[0, 0, 127, 0, 0, 47, 127, 0]);

    thread::spawn(move || {
        run_loop(&mut sequencer);
    });                                                                                                                        
                                                                                                                                              
    loop {                                                                                                                                    
        match rx.recv()? {                                                                                                                                                                                                                                      
            event::KeyEvent {                                                                                                                 
                code: KeyCode::Esc,                                                                                                           
                ..                                                                                                                            
            } => {                                                                                                                            
                break;                                                                                                                        
            },                                                                                                                                
            _ => {}                                                                                                                           
        }                                                                                                                                     
    }                                                                                                                                      
                                                                                                                                              
    // Cleanup                                                                                                                                
    terminal::disable_raw_mode()?;                                                                                                            
    execute!(stdout, LeaveAlternateScreen)?;                                                                                                  
    Ok(())                                                                                                                                    
} 