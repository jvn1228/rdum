use rodio::{OutputStreamHandle, Sink, Source};                                                                                     
use std::{sync::mpsc, time::Duration};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::time::Instant;

pub enum Division {
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

#[derive(Debug, Clone)]
pub struct TrackState {
    pub slots: Vec<u8>,
    pub name: String
}

#[derive(Debug)]
pub struct State {
    pub tempo: u8,
    pub trk_idx: usize,
    pub trks: Vec<TrackState>,
    pub division: u8,
    pub len: usize,
    pub latency: Duration
}

#[derive(Clone)]
pub struct BufferedSample {
    sample_rate: u32,
    channels: u16,
    current_sample: usize,
    buffer: Arc<Vec<f32>>,
}

impl BufferedSample {
    pub fn load_from_file(fp: &str) -> Result<BufferedSample, Box<dyn Error>> {
        let file = File::open(fp)?;
        let decoder = rodio::Decoder::new(file)?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let decoder = decoder.convert_samples::<f32>();
        let mut buffer = vec![];
        for d in decoder.buffered() {
            buffer.push(d);
        }
        let buffer = Arc::new(buffer);
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
        self.current_sample += 1;
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
            self.set_slot_vel(i, *v);
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
    state_ch: Vec<mpsc::Sender<State>>
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
        Ok(self.tracks.last().unwrap())
    }

    fn play_next(&mut self) {
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

    fn set_latency(&mut self, t: Duration) {
        self.latency = Duration::from_nanos(((self.latency + t).as_nanos() / 2) as u64);
        self.sleep_interval = self.pulse_interval - self.pulse_interval.min(self.latency);
    }

    pub fn set_division(&mut self, division: Division) {
        self.division = division as u8;
    }

    pub fn get_state_rx(&mut self) -> mpsc::Receiver<State> {
        let (tx, rx) = mpsc::channel();
        self.state_ch.push(tx);
        rx
    }

    fn tx_state(&self) {
        let trks: Vec<TrackState> = self.tracks.iter().map(|t| {
            TrackState {
                slots: t.slots.iter().map(|s| { *s.velocity.lock().unwrap() }).collect(),
                name: t.name.clone()
            }
        }).collect();
        for tx in &self.state_ch {
            let _ = tx.send(State {
                tempo: self.tempo,
                trk_idx: self.trk_idx,
                trks: trks.clone(),
                division: self.division,
                len: self.len,
                latency: self.latency
            });
        }
    }

    fn sleep(&self) {
        spin_sleep::sleep(self.sleep_interval);
    }

    pub fn run_loop(&mut self) {
        loop {
            self.play_next();
            self.sleep();
        }
    }
}