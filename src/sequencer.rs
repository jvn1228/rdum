use rodio::{OutputStreamHandle, Sink, Source};                                                                                     
use std::{sync::mpsc, time::Duration};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::time::Instant;
use std::thread::yield_now;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    SetTempo(u8),
    SetSlotVelocity(u8, u8),
    SetSequencerLength(usize),
    PlaySound(u8),
}
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
    pub latency: Duration,
    pub last_cmd: Command
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
    pub velocity: u8,
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
                velocity: 0
            });
        }
        Track {
            slots,
            sample,
            sink,
            name
        }
    }
}

pub struct Props {
    pub tracks: Vec<Track>,
    pub len: usize,
    tempo: u8,
    pulse_interval: Duration,
    division: u8,
    command_rx_ch: mpsc::Receiver<Command>,
    last_cmd: Command
}

impl Props {
    fn set_tempo(&mut self, bpm: u8) {
        self.tempo = bpm;
        self.pulse_interval = Duration::from_secs_f32(5.0 / 2.0 / bpm as f32);
    }
}

#[derive(Clone)]
pub struct PropsHandle {
    inner: Arc<Mutex<Props>>
}

impl PropsHandle {
    pub fn new(props: Props) -> Self {
        Self {
            inner: Arc::new(Mutex::new(props))
        }
    }

    pub fn with_lock<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Props) -> T,
    {
        let mut lock = self.inner.lock().unwrap();
        let result = func(&mut *lock);
        drop(lock);
        result
    }

    // we should put these methods on the props struct and just wrap for handler maybe?
    // so redundant though....
    pub fn set_tempo(&self, t: u8) {
        self.with_lock(|props| {
            props.set_tempo(t)
        })
    }

    pub fn division(&self) -> u8 {
        self.with_lock(|props| { props.division })
    }

    pub fn set_division(&self, division: Division) -> u8 {
        self.with_lock(|props| {
            props.division = division as u8;
            props.division
        })
    }
}

pub struct TrackHandle {
    inner: PropsHandle,
    trk: u8
}

impl TrackHandle {
    fn new(props_handle: PropsHandle, trk: u8) -> Self {
        Self {
            inner: props_handle,
            trk
        }
    }

    pub fn with_lock<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Track) -> T,
    {
        self.inner.with_lock(|props| {
            let t = &mut props.tracks[self.trk as usize];
            func(t)
        })
    }

    pub fn set_slot_vel(&self, slot: usize, vel: u8) {
        self.with_lock(|trk| {
            trk.slots[slot].velocity = vel;
        })
    }

    pub fn set_slots_vel(&self, vels: &[u8]) {
        self.with_lock(|trk| {
            for (i, v) in vels.iter().enumerate() {
                if i >= trk.slots.len() {
                    break;
                }
                trk.slots[i].velocity = *v;
            }
        })
    }
}

pub struct Sequencer {
    pub props: PropsHandle,
    pub stream: Arc<OutputStreamHandle>,
    pub trk_idx: usize,
    // Average of current and last cycle time
    latency: Duration,
    sleep_interval: Duration,
    // pulses per beat
    ppb: u8,
    pulse_idx: u8,
    state_tx_ch: Vec<mpsc::Sender<State>>,
    command_tx_ch: mpsc::Sender<Command>,
}

// Maybe tracks should have independent lengths?
impl Sequencer {
    pub fn new(len: usize, stream: Arc<OutputStreamHandle>) -> Sequencer {
        let (command_tx, command_rx) = mpsc::channel();
        Sequencer {
            props: PropsHandle::new(Props {
                tracks: vec![],
                len,
                tempo: 120,
                pulse_interval: Duration::from_secs_f32(1.0/12.0),
                // allowable set{1,2,3,4,6,8,12,16,24,32}
                division: 8,
                command_rx_ch: command_rx,
                last_cmd: Command::PlaySound(1)
            }),
            stream,
            trk_idx: 0,
            latency: Duration::ZERO,
            sleep_interval: Duration::from_secs_f32(1.0/24.0),
            // pulses per bar, 24 per quarter note
            ppb: 24*4,
            pulse_idx: 0,
            state_tx_ch: vec![],
            // command_rx_ch: Arc::new(command_rx),
            command_tx_ch: command_tx
        }
    }

    pub fn set_tempo(&mut self, bpm: u8) {
        self.props.set_tempo(bpm);
    }

    // Because trks are wrapped in a property handler to ensure thread safety, we can't directly return access
    // to the track. Instead the index of the created track is returned for reference
    pub fn add_track(&mut self, name: String, sample: Arc<BufferedSample>) -> Result<TrackHandle, Box<dyn Error>> {
        let sink = Sink::try_new(&self.stream)?;
        let sink = Arc::new(sink);
        sink.play();
        self.props.with_lock(|props| {
            props.tracks.push(Track::new(name, props.len, sample, sink));
            Ok(TrackHandle::new(self.props.clone(), props.tracks.len() as u8 - 1))
        })
    }

    fn play_next(&mut self) {
        let start = Instant::now();
        // hmm might have to create a spare vec of pulses where 1 is trigger to handle swing patterns
        // and then in fact we might have to move that tracking to the track
        if self.pulse_idx % (self.ppb / self.props.division()) == 0 {
            self.props.with_lock(|props| {
                for t in &mut props.tracks {
                    let vel = &mut t.slots[self.trk_idx].velocity;
                    if *vel > 0 {
                        t.sink.append((*t.sample).clone().amplify(*vel as f32 / 127.0));
                        if t.sink.len() > 1 {
                            t.sink.skip_one();
                        }
                    }
                }
                self.trk_idx = (self.trk_idx + 1) % props.len;
            })
        }
        self.pulse_idx = (self.pulse_idx + 1) % self.ppb;
        self.tx_state();
        
        // to do send midi clk msg
        self.set_latency(Instant::now().duration_since(start));
    }

    fn set_latency(&mut self, t: Duration) {
        self.latency = Duration::from_nanos(((self.latency + t).as_nanos() / 2) as u64);
        self.props.with_lock(|props| {
            self.sleep_interval = props.pulse_interval - props.pulse_interval.min(self.latency)
        })
    }

    pub fn set_division(&mut self, division: Division) {
        self.props.set_division(division);
    }

    pub fn get_state_rx(&mut self) -> mpsc::Receiver<State> {
        let (tx, rx) = mpsc::channel();
        self.state_tx_ch.push(tx);
        rx
    }

    pub fn get_command_tx(&mut self) -> mpsc::Sender<Command> {
        self.command_tx_ch.clone()
    }

    fn tx_state(&self) {
        self.props.with_lock(|props| {
            let trks: Vec<TrackState> = props.tracks.iter().map(|t| {
                TrackState {
                    slots: t.slots.iter().map(|s| { s.velocity }).collect(),
                    name: t.name.clone()
                }
            }).collect();
            for tx in &self.state_tx_ch {
                let _ = tx.send(State {
                    tempo: props.tempo,
                    trk_idx: self.trk_idx,
                    trks: trks.clone(),
                    division: props.division,
                    len: props.len,
                    latency: self.latency,
                    last_cmd: props.last_cmd
                });
            }
        })
    }

    pub fn run_command_loop(props: PropsHandle) {
        loop {
            props.with_lock(|props| {
                if let Ok(cmd) = props.command_rx_ch.try_recv() {
                    props.last_cmd = cmd;
                    match cmd {
                        Command::SetTempo(bpm) => props.set_tempo(bpm),
                        _ => ()
                    }
                } else {
                    // do nothing
                }
            });
            yield_now();
        }
    }

    fn sleep(&self) {
        spin_sleep::sleep(self.sleep_interval);
    }

    pub fn run_sound_loop(mut seq: Self) {
        loop {
            seq.play_next();
            seq.sleep();
            yield_now();
        }
    }

}