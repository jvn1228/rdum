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

enum Commands {
    SetTempo(f32),
    SetSlotVelocity(u8, u8),
    SetSequencerLength(usize),
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
    pub idx: usize,
    pub len: usize,
    // Average of current and last cycle time
    latency: Duration,
    tempo: u8,
    beat_interval: Duration,
    sleep_interval: Duration

}

impl Sequencer {
    pub fn new(len: usize, stream: Arc<OutputStreamHandle>) -> Sequencer {
        Sequencer {
            stream,
            tracks: vec![],
            idx: 0,
            len,
            tempo: 120,
            latency: Duration::ZERO,
            beat_interval: Duration::from_millis(500),
            sleep_interval: Duration::from_millis(500)
        }
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
        for t in &self.tracks {
            let vel = t.slots[self.idx].velocity.lock().unwrap();
            if *vel > 0 {
                t.sink.append((*t.sample).clone());
                if t.sink.len() > 1 {
                    t.sink.skip_one();
                }
            }
        }
        self.idx = (self.idx + 1) % self.len;
        self.set_latency(Instant::now().duration_since(start));
    }

    pub fn latency(&self) -> Duration {
        self.latency
    }

    fn set_latency(&mut self, t: Duration) {
        self.latency = Duration::from_nanos(((self.latency + t).as_nanos() / 2) as u64);
        self.sleep_interval = self.beat_interval - self.beat_interval.min(self.latency);
    }

    pub fn sleep(&self) {
        thread::sleep(self.sleep_interval);
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

    let sample_hat = BufferedSample::load_from_file(&format!("{pwd}/one_shots/hat0.wav").to_string())?;
    let sample_hat = Arc::new(sample_hat);
    let trk_hat = sequencer.add_track("Hat".to_string(), Arc::clone(&sample_hat))?;
    trk_hat.set_slots_vel(&[0, 127, 0, 127, 0, 127, 0, 127]);

    let sample_kick = BufferedSample::load_from_file(&format!("{pwd}/one_shots/kick0.wav").to_string())?;
    let sample_kick = Arc::new(sample_kick);
    let trk_kick = sequencer.add_track("Kick".to_string(), Arc::clone(&sample_kick))?;
    trk_kick.set_slots_vel(&[127, 0, 127, 127, 0, 127, 0, 127]);

    let sample_snare = BufferedSample::load_from_file(&format!("{pwd}/one_shots/snare0.wav").to_string())?;
    let sample_snare = Arc::new(sample_snare);
    let trk_snare = sequencer.add_track("Snare".to_string(), Arc::clone(&sample_snare))?;
    trk_snare.set_slots_vel(&[0, 0, 127, 0, 0, 0, 127, 0]);

    thread::spawn(move || {
        run_loop(&mut sequencer);
    });                                                                                                                        
                                                                                                                                              
    loop {                                                                                                                                    
        match rx.recv()? {                                                                                                                    
            // event::KeyEvent {                                                                                                                 
            //     code: KeyCode::Char(' '),                                                                                                     
            //     ..                                                                                                                            
            // } => {
            //     if sink.is_paused() {                                                                                                         
            //         sink.play();                                                                                                              
            //     }                                                                                                                       
            //     if sink.empty() {                                                             
            //         sink.append(sample.decoder().convert_samples::<f32>());                                                                                              
            //     } else {        
            //         sink.append(sample.decoder().convert_samples::<f32>());                                                                                                                                                                                                                     
            //         sink.skip_one();                                                          
            //     }                                                                                                                                                                                                                                                        
            // },
            // event::KeyEvent {                                                                                                                 
            //     code: KeyCode::Char('a'),                                                                                                     
            //     ..                                                                                                                            
            // } => {
            //     if sink2.is_paused() {                                                                                                         
            //         sink2.play();                                                                                                              
            //     }                                                                                                                       
            //     if sink2.empty() {                                                             
            //         sink2.append(s2.decoder().convert_samples::<f32>());                                                                                              
            //     } else {        
            //         sink2.append(s2.decoder().convert_samples::<f32>());                                                                                                                                                                                                                     
            //         sink2.skip_one();                                                          
            //     }                                                                                                                                                                                                                                                        
            // },
            // event::KeyEvent {                                                                                                                 
            //     code: KeyCode::Char('d'),                                                                                                     
            //     ..                                                                                                                            
            // } => {
            //     if sink3.is_paused() {                                                                                                         
            //         sink3.play();                                                                                                              
            //     }                                                                                                                       
            //     if sink3.empty() {                                                             
            //         sink3.append(s3.decoder().convert_samples::<f32>());                                                                                              
            //     } else {        
            //         sink3.append(s3.decoder().convert_samples::<f32>());                                                                                                                                                                                                                     
            //         sink3.skip_one();                                                          
            //     }                                                                                                                                                                                                                                                        
            // },                                                                                                                    
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