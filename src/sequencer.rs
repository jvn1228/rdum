use rodio::{OutputStreamHandle, Sink, Source};                                                                                     
use std::{sync::mpsc, time::Duration};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::time::Instant;
use std::thread::yield_now;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Command {
    // Sequencer playback commands
    PlaySequencer,
    StopSequencer,
    SetTempo(u8),
    SetPattern(u8),
    SetDivision(Division),
    PlaySound(usize, u8),
    // Track program commands
    SetSlotVelocity(usize, usize, u8),
    SetTrackLength(usize),
    // Sequencer program commands
    AddPattern,
    RemovePattern(u8),
    Unspecified,
}

impl Default for Command {
    fn default() -> Self { Command::Unspecified }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
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

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TrackState {
    pub slots: Vec<u8>,
    pub name: String,
    pub len: usize,
    pub idx: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
/// Subset of sequencer state that be broadcast on a channel
/// 
/// Refer to the Props struct to see more descriptors
pub struct State {
    pub tempo: u8,
    pub trks: Vec<TrackState>,
    pub division: u8,
    pub default_len: usize,
    pub latency: Duration,
    pub last_cmd: Command,
    pub playing: bool,
    pub pattern_idx: usize,
    pub pattern_len: usize,
    pub pattern_name: String,
}

#[derive(Clone)]
/// BufferedSample is a custom Rodio source that holds
/// the decoded sample data in memory. So it's much faster
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

/// `Track` contains data that allows the sequencer to play a sample 
/// 
/// It has a vector of velocities that determine when a sample is triggered, an audio sink to queue it,
/// and a reference to the sample itself
/// Tracks also can have their own length, leading to interesting pattern variations
pub struct Track {
    pub slots: Vec<Slot>,
    pub sample: Arc<BufferedSample>,
    pub idx: usize,
    pub len: usize,
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
            idx: 0, 
            len,
            sink,
            name
        }
    }
}

pub struct ChokeGrp {
    pub track_idxs: Vec<usize>,
}

impl ChokeGrp {
    pub fn new(tracks: Vec<usize>) -> ChokeGrp {
        ChokeGrp {
            track_idxs: tracks
        }
    }

    pub fn add_track(&mut self, track: usize) {
        self.track_idxs.push(track);
    }

    pub fn remove_track(&mut self, track_idx: usize) {
        self.track_idxs.retain(|&x| x != track_idx);
    }

    pub fn is_member(&self, track_idx: usize) -> bool {
        self.track_idxs.contains(&track_idx)
    }

    // Returns list of other indices if a given track index is
    // in the choke group
    pub fn get_choked_idxs(&self, track_idx: usize) -> Vec<usize> {
        if !self.is_member(track_idx) {
            return vec![];
        }
        self.track_idxs.iter().filter(|&&x| x != track_idx).cloned().collect()
    }
}

/// `Pattern` is a collection of tracks
/// 
/// To do: this probably needs to wrap as a props handle as well
pub struct Pattern {
    pub tracks: Vec<Track>,
    pub choke_grps: Vec<ChokeGrp>,
    pub name: String,
}

impl Pattern {
    // Returns list of other track indices across all groups that the given track index chokes
    pub fn get_choked_idxs(&self, track_idx: usize) -> Vec<usize> {
        let mut idxs = self.choke_grps
            .iter()
            .filter(|choke_grp| choke_grp.is_member(track_idx))
            .flat_map(|choke_grp| choke_grp.get_choked_idxs(track_idx))
            .collect::<Vec<usize>>();
        idxs.dedup();
        idxs
    }

    // Returns true if the track is in a choke group with any triggered tracks
    pub fn is_trk_choked(&self, triggered_idxs: &Vec<usize>, track_idx: usize) -> bool {
        triggered_idxs
            .iter()
            .any(|&x| self.get_choked_idxs(x).contains(&track_idx))
    }
}

/// Struct that describes internal sequencer state that can be
/// modified by the user
pub struct Props {
    pub patterns: Vec<Pattern>,
    pub pattern_idx: usize,
    /// It's the default length of a new track, unit is beats
    pub default_len: usize,
    /// beats per minutes
    tempo: u8,
    /// calculated based on tempo, the length of one pulse of the sequencer
    /// 
    /// note: this is not the same as a beat and has to be a higher frequency
    /// to handle things like swing
    pulse_interval: Duration,
    /// defines the note length of a beat
    /// 
    /// allowable set{1,2,3,4,6,8,12,16,24,32}
    division: u8,
    playing: bool,
    command_rx_ch: mpsc::Receiver<Command>,
    last_cmd: Command
}

impl Props {
    fn set_tempo(&mut self, bpm: u8) {
        self.tempo = bpm;
        self.pulse_interval = Duration::from_secs_f32(5.0 / 2.0 / bpm as f32);
    }

    pub fn enable_play(&mut self) {
        self.playing = true;
    }

    pub fn disable_play(&mut self) {
        self.playing = false;
    }

    pub fn set_division(&mut self, division: Division) {
        self.division = division as u8;
    }
}

/// Struct wrapping sequencer Props allowing us to modify them
/// without taking ownership of them
/// 
///  Borrowing/ownership and race conditions present some challenges in multithreaded apps,
/// the solution of prop handlers is used here to solve them
/// The wrapper will take care of mutex locks and allows many threads to safely access the struct
/// without violating ownership principles (An Arc smart pointer is used)
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

    pub fn set_division(&self, division: Division) {
        self.with_lock(|props| {
            props.set_division(division);
        })
    }

    pub fn enable_play(&mut self) {
        self.with_lock(|props| {
            props.enable_play();
        })
    }

    pub fn disable_play(&mut self) {
        self.with_lock(|props| {
            props.disable_play();
        })
    }
}

/// Struct that wraps PropsHandle for a specific track
/// 
///  This is a little clunky but because sequencer Props are
/// defined as the user-modifiable properties, which includes tracks,
/// we have to use the PropsHandle wrapper to access the tracks
/// TrackHandle serves as a specialized PropsHandle that is just
/// for modifying tracks. But, clunkily, it's a wrapper of a wrapper
/// One day maybe aspiring rappers will appreciate this wrapper wrapper.
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
            let t = &mut props
                .patterns[props.pattern_idx]
                .tracks[self.trk as usize];
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

/// `Sequencer` is the main sound engine
/// 
/// The hierarchy looks like this: Sequencer -> Pattern -> Track -> Sample
/// When playing, the sequencer keeps track of the current playhead positions,
/// triggering samples loaded into the individual tracks based on the
/// track's vector of sample velocities
/// It runs at a higher refresh rate (pulse) than the beat since it can
/// also send midi clock signals and handle swung notes
/// The sequencer can be controlled by creating a command channel and
/// controllers/displays can receive state on a state broadcast channel
pub struct Sequencer {
    /// Properties that can be modified
    pub props: PropsHandle,
    pub stream: Arc<OutputStreamHandle>,
    /// Average of current and last cycle time
    latency: Duration,
    /// the actual sleep time, which may differ from pulse interval
    /// if, for example, processing latency is high
    sleep_interval: Duration,
    // pulses per bar, always gonna be 24*4 for midi clock purposes
    ppb: u8,
    pulse_idx: u8,
    /// State transmission channel
    /// 
    /// Unfortunately the current standard Rust channel only
    /// allows for a single consumer, so we can't broadcast state
    /// updates to many listeners except via multiple channels
    state_tx_ch: Vec<mpsc::Sender<State>>,
    /// Command receiver channel
    /// 
    /// Multi producer single consumer means we can
    /// have multiple controllers (producers) on the sequencer (consumer) at once
    command_tx_ch: mpsc::Sender<Command>,
}

// Maybe tracks should have independent lengths?
impl Sequencer {
    /// Creates a new sequencer instance
    pub fn new(stream: Arc<OutputStreamHandle>) -> Sequencer {
        let (command_tx, command_rx) = mpsc::channel();
        Sequencer {
            props: PropsHandle::new(Props {
                patterns: vec![Pattern {
                    tracks: vec![],
                    choke_grps: vec![],
                    name: "Pattern 1".to_string()
                }],
                pattern_idx: 0,
                default_len: 8,
                tempo: 120,
                // corresponds to 120 bpm
                pulse_interval: Duration::from_secs_f32(2.5/120.0),
                division: 4,
                playing: false,
                command_rx_ch: command_rx,
                last_cmd: Command::Unspecified
            }),
            stream,
            latency: Duration::ZERO,
            sleep_interval: Duration::from_secs_f32(1.0/24.0),
            // pulses per bar, 24 per quarter note
            // afaik this is the rate to send midi clock signals
            ppb: 24*4,
            pulse_idx: 0,
            state_tx_ch: vec![],
            command_tx_ch: command_tx
        }
    }

    /// Sets tempo via props handle
    pub fn set_tempo(&mut self, bpm: u8) {
        self.props.set_tempo(bpm);
    }

    pub fn play(&mut self) {
        self.props.enable_play();
    }

    pub fn stop(&mut self) {
        self.props.disable_play();
    }

    /// Adds an empty track to the sequencer at the current pattern
    /// 
    /// Because trks are wrapped in a property handler to ensure thread safety, we can't directly return access
    /// to the track. Instead the index of the created track is returned for reference
    pub fn add_track(&mut self, name: String, sample: Arc<BufferedSample>) -> Result<TrackHandle, Box<dyn Error>> {
        let sink = Sink::try_new(&self.stream)?;
        let sink = Arc::new(sink);
        sink.play();
        self.props.with_lock(|props| {
            let tracks = &mut props.patterns[props.pattern_idx].tracks;
            tracks.push(Track::new(name, props.default_len, sample, sink));
            Ok(TrackHandle::new(self.props.clone(), tracks.len() as u8 - 1))
        })
    }

    /// Helper function that plays a sample on the playback stream sink
    /// 
    /// We circumvent the rodio sink queueing, only instant plays! It's a little clunky perhaps to repeatedly clone
    /// the Arc pointer but optimization is a later thing
    fn append_sample_to_sink(snk: Arc<Sink>, samp: Arc<BufferedSample>, vel: &mut u8) {
        snk.append((*samp).clone().amplify(*vel as f32 / 127.0));
        if snk.len() > 1 {
            snk.skip_one();
        }
    }

    /// Resets all track indices to 0
    fn reset_playheads(&mut self) {
        self.props.with_lock(|props| {
            let tracks = &mut props.patterns[props.pattern_idx].tracks;
            for t in tracks {
                t.idx = 0;
            }
        })
    }

    /// The VIP function. Plays tracks, sends state, updates latency
    fn play_next(&mut self) {
        let playing = self.props.with_lock(|props| { props.playing });
        if playing {
            let start = Instant::now();
            // hmm might have to create a spare vec of pulses where 1 is trigger to handle swing patterns
            // and then in fact we might have to move that tracking to the track
            if self.pulse_idx % (self.ppb / self.props.division()) == 0 {
                self.props.with_lock(|props| {
                    let mut triggered_idxs: Vec<usize> = vec![];
                    let pattern = &mut props.patterns[props.pattern_idx];
                    let tracks = &mut pattern.tracks;
                    for (i, t) in tracks.into_iter().enumerate() {
                        let vel = &mut t.slots[t.idx].velocity;
                        if *vel > 0 {
                            Sequencer::append_sample_to_sink(t.sink.clone(), t.sample.clone(), vel);
                            triggered_idxs.push(i);
                        }

                        t.idx = (t.idx + 1) % t.len;
                    }
                    
                    // Redefine as immutable to prevent triggering borrow checker
                    let pattern = &props.patterns[props.pattern_idx];
                    let tracks = &pattern.tracks;
                    for i in 0..tracks.len() {
                        if pattern.is_trk_choked(&triggered_idxs, i) {
                            tracks[i].sink.skip_one();
                        }
                    }
                })
            }
            self.pulse_idx = (self.pulse_idx + 1) % self.ppb;
            
            // to do send midi clk msg
            self.set_latency(Instant::now().duration_since(start));

        } else if self.pulse_idx != 0 {
            self.pulse_idx = 0;
            self.reset_playheads();
        }

        self.tx_state();
    }

    /// Attempts to keep timing tight by subtracting processing time from overall wait between beats
    fn set_latency(&mut self, t: Duration) {
        self.latency = Duration::from_nanos(((self.latency + t).as_nanos() / 2) as u64);
        self.props.with_lock(|props| {
            self.sleep_interval = props.pulse_interval - props.pulse_interval.min(self.latency)
        })
    }

    /// Uses props handle to set time division (4/4 time is quarter division, 4/8 is eighth, etc)
    pub fn set_division(&mut self, division: Division) {
        self.props.set_division(division);
    }

    /// Creates a new channel to send state updates to
    pub fn get_state_rx(&mut self) -> mpsc::Receiver<State> {
        let (tx, rx) = mpsc::channel();
        self.state_tx_ch.push(tx);
        rx
    }

    /// Creates a command tx channel to receive commands
    /// 
    /// If multiple controllers are used, no attempt is made to counteract
    /// race conditions between them, sequencer only receive commands one at a time
    pub fn get_command_tx(&mut self) -> mpsc::Sender<Command> {
        self.command_tx_ch.clone()
    }

    /// Transmits a subset of internal sequencer state
    fn tx_state(&self) {
        self.props.with_lock(|props| {
            let trks: Vec<TrackState> = props
                .patterns[props.pattern_idx]
                .tracks
                .iter()
                .map(|t| {
                    TrackState {
                        slots: t.slots.iter().map(|s| { s.velocity }).collect(),
                        name: t.name.clone(),
                        idx: t.idx,
                        len: t.len
                    }
                })
                .collect();

            for tx in &self.state_tx_ch {
                let _ = tx.send(State {
                    tempo: props.tempo,
                    trks: trks.clone(),
                    division: props.division,
                    default_len: props.default_len,
                    latency: self.latency,
                    last_cmd: props.last_cmd,
                    playing: props.playing,
                    pattern_idx: props.pattern_idx,
                    pattern_len: props.patterns.len(),
                    pattern_name: props.patterns[props.pattern_idx].name.clone(),
                });
            }
        })
    }

    /// Receives commands and modifies sequencer state accordingly
    /// 
    /// You can run this in its own thread. It does not own the sequencer
    /// instance hence we use a props handle to modify the sequencer state
    /// There's a slight weirdness with this paradigm in that one shot
    /// sample playing will directly add to the track playback sink, instead
    /// of modifying a property. Maybe tracks are not fully definable as properties
    /// but we gain functionality treating them as such
    pub fn run_command_loop(props: PropsHandle) {
        loop {
            props.with_lock(|props| {
                if let Ok(cmd) = props.command_rx_ch.try_recv() {
                    props.last_cmd = cmd;
                    match cmd {
                        Command::SetTempo(bpm) => props.set_tempo(bpm),
                        Command::PlaySound(trk, vel) => (|trk, vel| {
                                let trk: &mut Track = &mut props.patterns[props.pattern_idx].tracks[trk];
                                let mut vel = vel;
                                let v = &mut vel;
                                Sequencer::append_sample_to_sink(trk.sink.clone(), trk.sample.clone(), v);
                            })(trk, vel),
                        Command::PlaySequencer => props.enable_play(),
                        Command::StopSequencer => props.disable_play(),
                        Command::SetDivision(div) => props.set_division(div),
                        Command::SetSlotVelocity(trk, slot, vel) => {
                            props.patterns[props.pattern_idx].tracks[trk].slots[slot].velocity = vel;
                        },
                        _ => ()
                    }
                } else {
                    // do nothing
                }
            });
            yield_now();
        }
    }

    /// Sleep between pulses
    fn sleep(&self) {
        spin_sleep::sleep(self.sleep_interval);
    }

    /// Runs the sequencer
    pub fn run_sound_loop(mut seq: Self) {
        loop {
            seq.play_next();
            seq.sleep();
            yield_now();
        }
    }

}