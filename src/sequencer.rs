use rodio::{OutputStreamHandle, Sink, Source};
use tokio::time::error::Elapsed;                                                                                     
use std::{sync::mpsc, time::Duration};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::fs::{File, OpenOptions};
use std::time::Instant;
use std::thread::yield_now;
use midir::{MidiOutput, MidiOutputPort, MidiOutputConnection};
use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};

const PWD: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone)]
pub enum StateUpdate {
    FileState(FileState),
    SeqState(SeqState),
}

#[derive(Debug, Clone, Serialize)]
pub enum FileType {
    #[serde(rename = "pattern")]
    Pattern,
    #[serde(rename = "sample")]
    Sample,
}

/// Struct that allows updating listeners of samples
/// and saved patterns
#[derive(Debug, Clone, Serialize)]
pub struct FileState {
    #[serde(rename = "type")]
    pub file_type: FileType,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Command {
    // Sequencer playback commands
    PlaySequencer,
    StopSequencer,
    SetTempo(u8),
    SetPattern(usize),
    PlaySound(usize, u8),
    // Track program commands
    SetSlotVelocity(usize, usize, u8),
    SetTrackLength(usize),
    // Sequencer program commands
    AddPattern,
    RemovePattern(usize),
    SelectPattern(usize),
    SetPatternLength(usize),
    SavePattern,
    LoadPattern(String),
    // A single controller can request this but due
    // to state update patterns, all controllers
    // will receive the update
    ListPatterns,
    ListSamples,
    // Pattern program commands
    SetDivision(Division),
    AddTrack(String),
    SetTrackSample(usize, String),
    Unspecified,
}

impl Default for Command {
    fn default() -> Self { Command::Unspecified }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash)]
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

impl From<i64> for Division {
    fn from(value: i64) -> Self {
        match value {
            1 => Division::W,
            2 => Division::H,
            3 => Division::QD,
            4 => Division::Q,
            6 => Division::ED,
            8 => Division::E,
            12 => Division::SD,
            16 => Division::S,
            24 => Division::TD,
            32 => Division::T,
            _ => Division::W,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TrackState {
    pub slots: Vec<u8>,
    pub name: String,
    pub len: usize,
    pub idx: usize,
    pub sample_path: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
/// Subset of sequencer state that be broadcast on a channel
/// 
/// Refer to the Context struct to see more descriptors
pub struct SeqState {
    pub tempo: u8,
    pub trks: Vec<TrackState>,
    pub division: u8,
    pub default_len: usize,
    pub latency: Duration,
    pub last_cmd: Command,
    pub playing: bool,
    pub pattern_id: usize,
    pub pattern_len: usize,
    pub pattern_name: String,
    pub queued_pattern_id: usize,
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
    fn new(fp: &str) -> Result<Arc<Self>, Box<dyn Error>> {
        let sample = Self::load_from_file(&format!("{PWD}/samples/{fp}").to_string())?;
        Ok(Arc::new(sample))
    }

    pub fn load_from_file(fp: &str) -> Result<Self, Box<dyn Error>> {
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

#[derive(Clone, Serialize, Deserialize, Hash)]
pub struct Slot {
    pub velocity: u8,
}

/// Struct for saving track data to file
#[derive(Clone, Serialize, Deserialize, Hash)]
pub struct SavedTrack {
    pub slots: Vec<Slot>,
    pub sample_path: String,
}

/// `Track` contains data that allows the sequencer to play a sample 
/// 
/// It has a vector of velocities that determine when a sample is triggered, an audio sink to queue it,
/// and a reference to the sample itself
/// Tracks also can have their own length, leading to interesting pattern variations
#[derive(Clone)]
pub struct Track {
    pub slots: Vec<Slot>,
    pub sample: Arc<BufferedSample>,
    pub sample_path: String,
    pub idx: usize,
    pub len: usize,
    pub sink: Arc<Sink>,
    pub name: String,
}

impl Track {
    pub fn new(len: usize, sample_path: String, sink: Arc<Sink>) -> Result<Self, Box<dyn Error>> {
        let name = sample_path.split('/').last().unwrap().split('.').next().unwrap().to_string();
        let mut slots = vec![];
        for _ in 0..len {
            slots.push(Slot {
                velocity: 0
            });
        }
        let sample = BufferedSample::new(&sample_path)?;
        Ok(Track {
            slots,
            sample,
            sample_path,
            idx: 0, 
            len,
            sink,
            name
        })
    }

    pub fn reset_slots(&mut self) {
        self.slots.iter_mut().for_each(|slot| {
            slot.velocity = 0;
        });
    }

    pub fn set_len(&mut self, len: usize) {
        if len > self.len {
            self.slots.extend(vec![Slot { velocity: 0 }; len - self.len]);
        } else {
            self.slots.truncate(len);
            self.idx = self.idx % len;
        }
        self.len = len;
    }

    pub fn set_sample(&mut self, sample_path: String) -> Result<(), Box<dyn Error>> {
        let sample = BufferedSample::new(&sample_path)?;
        self.sample = sample;
        self.sample_path = sample_path;
        Ok(())
    }
}

/// ChokeGrp allows defining tracks that stop other tracks in
/// the same choke group when triggered
#[derive(Clone, Serialize, Deserialize, Hash)]
pub struct ChokeGrp {
    pub track_ids: Vec<usize>,
}

impl ChokeGrp {
    pub fn new(tracks: Vec<usize>) -> ChokeGrp {
        ChokeGrp {
            track_ids: tracks
        }
    }

    pub fn add_track(&mut self, track_id: usize) {
        self.track_ids.push(track_id);
    }

    pub fn remove_track(&mut self, track_id: usize) {
        self.track_ids.retain(|&x| x != track_id);
    }

    pub fn is_member(&self, track_id: usize) -> bool {
        self.track_ids.contains(&track_id)
    }

    // Returns list of other indices if a given track index is
    // in the choke group
    pub fn get_choked_ids(&self, track_id: usize) -> Vec<usize> {
        if !self.is_member(track_id) {
            return vec![];
        }
        self.track_ids.iter().filter(|&&x| x != track_id).cloned().collect()
    }
}

/// Struct for saving pattern data to file
#[derive(Clone, Serialize, Deserialize, Hash)]
pub struct SavedPattern {
    pub tracks: Vec<SavedTrack>,
    pub choke_grps: Vec<ChokeGrp>,
    pub division: Division
}

/// `Pattern` is a collection of tracks
/// 
/// If an empty pattern is saved, this can be considered a kit.
#[derive(Clone)]
pub struct Pattern {
    pub tracks: Vec<Track>,
    pub choke_grps: Vec<ChokeGrp>,
    /// defines the note length of a beat
    /// 
    /// allowable set{1,2,3,4,6,8,12,16,24,32}
    pub division: Division,
    pub name: String,
}

impl Pattern {
    // Returns list of other track indices across all groups that the given track index chokes
    pub fn get_choked_ids(&self, track_id: usize) -> Vec<usize> {
        let mut choked_ids = self.choke_grps
            .iter()
            .filter(|choke_grp| choke_grp.is_member(track_id))
            .flat_map(|choke_grp| choke_grp.get_choked_ids(track_id))
            .collect::<Vec<usize>>();
        choked_ids.dedup();
        choked_ids
    }

    // Returns true if the track is in a choke group with any triggered tracks
    pub fn is_trk_choked(&self, triggered_ids: &Vec<usize>, track_id: usize) -> bool {
        triggered_ids
            .iter()
            .any(|&x| self.get_choked_ids(x).contains(&track_id))
    }

    pub fn zero_all_tracks(&mut self) {
        self.tracks.iter_mut().for_each(|track| {
            track.reset_slots();
        });
    }

    pub fn reset_playheads(&mut self) {
        self.tracks.iter_mut().for_each(|track| {
            track.idx = 0;
        });
    }

    pub fn set_len(&mut self, len: usize) {
        self.tracks.iter_mut().for_each(|track| {
            track.set_len(len);
        });
    }

    pub fn set_division(&mut self, division: Division) {
        self.division = division;
    }

    // sample_path is the relative location of the sample file to the samples directory
    // This behavior is hardcoded for now
    pub fn add_track(&mut self, stream: Arc<OutputStreamHandle>, len: usize, sample_path: String) -> Result<(), Box<dyn Error>> {
        let sink = Sink::try_new(&stream)?;
        let sink = Arc::new(sink);
        sink.play();
        let tracks = &mut self.tracks;
        tracks.push(Track::new(len, sample_path, sink)?);
        Ok(())
    }

    pub fn set_track_sample(&mut self, track_id: usize, sample_path: String) -> Result<(), Box<dyn Error>> {
        self.tracks[track_id].set_sample(sample_path)
    }
}

/// Struct that describes internal sequencer state that can be
/// modified by the user as well as connections and channels
/// 
/// Note that many parameters are actually pattern-specific
pub struct Context {
    pub stream: Arc<OutputStreamHandle>,
    pub patterns: Vec<Pattern>,
    pub saved_patterns: Vec<String>,
    pub sample_files: Vec<String>,
    pub pattern_id: usize,
    // If there is no new pattern for queueing it should be
    // the current pattern since the same pattern is queued
    // for playing next
    pub queued_pattern_id: usize,
    /// It's the default length of a new track, unit is beats
    pub default_len: usize,
    /// beats per minutes
    tempo: u8,
    /// calculated based on tempo, the length of one pulse of the sequencer
    /// 
    /// note: this is not the same as a beat and has to be a higher frequency
    /// to handle things like swing
    pulse_interval: Duration,
    playing: bool,
    command_rx_ch: mpsc::Receiver<Command>,
    last_cmd: Command,
    pub midi_conn: Option<Arc<MidiOutputConnection>>,
    /// State transmission channel
    /// 
    /// Unfortunately the current standard Rust channel only
    /// allows for a single consumer, so we can't broadcast state
    /// updates to many listeners except via multiple channels
    state_tx_ch: Vec<mpsc::Sender<StateUpdate>>,
}

impl Context {
    fn set_tempo(&mut self, bpm: u8) {
        self.tempo = bpm;
        self.pulse_interval = Duration::from_secs_f32(5.0 / 2.0 / bpm as f32);
    }

    pub fn enable_play(&mut self) {
        self.playing = true;
        if let Some(midi_conn) = &mut self.midi_conn {
            let conn = Arc::<MidiOutputConnection>::get_mut(midi_conn).unwrap();
            conn.send(&[0xFA]).unwrap();
        }
    }

    pub fn disable_play(&mut self) {
        self.playing = false;
        if let Some(midi_conn) = &mut self.midi_conn {
            let conn = Arc::<MidiOutputConnection>::get_mut(midi_conn).unwrap();
            conn.send(&[0xFC]).unwrap();
        }
    }

    pub fn reset_playheads(&mut self) {
        self.patterns[self.pattern_id].reset_playheads();
    }

    // Saves the current pattern with named after its index
    // We also save a shortened hash of the file with it
    // but todo, I do think we need to allow specifying a name
    // or the user will get lost
    pub fn save_pattern(&mut self) -> Result<(), Box<dyn Error>> {
        let pattern = &self.patterns[self.pattern_id];
        let saved_pattern = SavedPattern {
            tracks: pattern.tracks.iter().map(|track| SavedTrack {
                slots: track.slots.clone(),
                sample_path: track.sample_path.clone()
            }).collect(),
            choke_grps: pattern.choke_grps.clone(),
            division: pattern.division,
        };
        let mut hash = std::hash::DefaultHasher::new();
        saved_pattern.hash(&mut hash);
        // converts to hex and truncates
        let hash = format!("{:x}", hash.finish())[..8].to_string();
        let f_name = format!("{}-{}.json", pattern.name, hash);
        let f_name = f_name.replace(" ", "_");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(format!("{PWD}/patterns/{}", &f_name))?;
        let file = std::io::BufWriter::new(file);
        serde_json::to_writer(file, &saved_pattern)?;
        self.refresh_saved_patterns()?;
        Ok(())
    }

    // Loads pattern from json file
    // This creates a new sink, and I am not sure old sinks are
    // destroyed when added to the stream so...maybe the better way I suspect
    // is to rotate available sinks
    pub fn load_pattern(&mut self, pattern_fname: String) -> Result<(), Box<dyn Error>> {
        let pattern = &self.patterns[self.pattern_id];

        let file = std::fs::File::open(format!("{PWD}/patterns/{}", pattern_fname))?;
        let file = std::io::BufReader::new(file);
        let saved_pattern: SavedPattern = serde_json::from_reader(file)?;

        self.patterns[self.pattern_id] = Pattern {
            tracks: saved_pattern.tracks.iter().filter_map(
                |track|
                if let Ok(sink) = Sink::try_new(&self.stream) {
                    if let Ok(mut t) = Track::new(
                        track.slots.len(),
                        track.sample_path.clone(),
                        Arc::new(sink)
                    ) {
                        t.slots = track.slots.clone();
                        Some(t)
                    } else {
                        None
                    }
                } else {
                    None
                }
            ).collect(),
            choke_grps: saved_pattern.choke_grps.clone(),
            division: saved_pattern.division,
            name: pattern.name.clone(),
        };
        if self.playing {
            // just so we send a midi start message out
            self.enable_play();
        }
        Ok(())
    }

    pub fn refresh_saved_patterns(&mut self) -> Result<(), Box<dyn Error>> {
        let patterns = std::fs::read_dir(format!("{PWD}/patterns"))?;
        let patterns = patterns.filter_map(|entry| {
            if let Ok(entry) = entry {
                if let Some(path) = entry.path().to_str() {
                    // Only return file name
                    Some(path.split('/').last().unwrap().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }).collect();
        self.saved_patterns = patterns;
        self.send_file_state(FileType::Pattern);
        Ok(())
    }

    // Iterates through samples folder, including subfolders in the path to better
    // help organize the files into kits.
    pub fn refresh_sample_files(&mut self) -> Result<(), Box<dyn Error>> {
        let samples = std::fs::read_dir(format!("{PWD}/samples"))?;
        let samples = samples.filter_map(|entry| {
            if let Ok(entry) = entry {
                // If it's a directory, we need to iterate through it
                // and get a vector of paths like subfolder/file.wav
                if entry.path().is_dir() {
                    let subfolder = entry.path().to_str().unwrap().split('/').last().unwrap().to_string();
                    if let Ok(files) = std::fs::read_dir(entry.path()) {
                        let files = files.filter_map(|entry| {
                            if let Ok(entry) = entry {
                                if let Some(path) = entry.path().to_str() {
                                    if entry.path().is_file() {
                                        Some(format!("{}/{}", subfolder, path.split('/').last().unwrap()))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }).collect::<Vec<String>>();
                        Some(files)
                    } else {
                        None
                    }
                // It's just a file
                } else {
                    if let Some(path) = entry.path().to_str() {
                        if entry.path().is_file() {
                            Some(vec![path.to_string().split('/').last().unwrap().to_string()])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        })
        .flatten()
        .collect();

        self.sample_files = samples;
        self.send_file_state(FileType::Sample);

        Ok(())
    }

    /// Sends special state update for files only
    /// This can be triggered if changes occurred in the file system
    /// Also yes, yes the other state tx is in sequencer and I'm beginning
    /// to think we should just lock the whole sequencer and forget
    /// about the context
    pub fn send_file_state(&self, file_type: FileType) {
        for tx in &self.state_tx_ch {
            let _ = tx.send(StateUpdate::FileState(FileState {
                file_type: file_type.clone(),
                files: match file_type {
                    FileType::Pattern => self.saved_patterns.clone(),
                    FileType::Sample => self.sample_files.clone(),
                },
            }));
        }
    }
}

/// Struct wrapping sequencer Context allowing us to modify it
/// without taking ownership of it
/// 
///  Borrowing/ownership and race conditions present some challenges in multithreaded apps,
/// the solution of prop handlers is used here to solve them
/// The wrapper will take care of mutex locks and allows many threads to safely access the struct
/// without violating ownership principles (An Arc smart pointer is used)
#[derive(Clone)]
pub struct ContextHandle {
    inner: Arc<Mutex<Context>>
}

impl ContextHandle {
    pub fn new(ctx: Context) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ctx))
        }
    }

    pub fn with_lock<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Context) -> T,
    {
        let mut lock = self.inner.lock().unwrap();
        let result = func(&mut *lock);
        drop(lock);
        result
    }

    // we should put these methods on the ctx struct and just wrap for handler maybe?
    // so redundant though....
    pub fn set_tempo(&self, t: u8) {
        self.with_lock(|ctx| {
            ctx.set_tempo(t)
        })
    }

    pub fn enable_play(&mut self) {
        self.with_lock(|ctx| {
            ctx.enable_play();
        })
    }

    pub fn disable_play(&mut self) {
        self.with_lock(|ctx| {
            ctx.disable_play();
        })
    }
}

/// Struct that wraps ContextHandle for a specific track
/// 
/// Clunkily, it's a wrapper of a wrapper
/// One day maybe aspiring rappers will appreciate this wrapper wrapper.
pub struct TrackHandle {
    inner: ContextHandle,
    id: u8
}

impl TrackHandle {
    fn new(ctx_handle: ContextHandle, id: u8) -> Self {
        Self {
            inner: ctx_handle,
            id
        }
    }

    pub fn with_lock<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Track) -> T,
    {
        self.inner.with_lock(|ctx| {
            let t = &mut ctx
                .patterns[ctx.pattern_id]
                .tracks[self.id as usize];
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
    pub ctx: ContextHandle,
    /// Average of current and last cycle time
    latency: Duration,
    /// the actual sleep time, which may differ from pulse interval
    /// if, for example, processing latency is high
    sleep_interval: Duration,
    // pulses per bar, always gonna be 24*4 for midi clock purposes
    ppb: u8,
    pulse_idx: u8,
    /// Command receiver channel
    /// 
    /// Multi producer single consumer means we can
    /// have multiple controllers (producers) on the sequencer (consumer) at once
    command_tx_ch: mpsc::Sender<Command>,
    sleeper: spin_sleep::SpinSleeper,
}

// Maybe tracks should have independent lengths?
impl Sequencer {
    /// Creates a new sequencer instance
    pub fn new(stream: Arc<OutputStreamHandle>) -> Sequencer {
        let (command_tx, command_rx) = mpsc::channel();
        let s = Sequencer {
            ctx: ContextHandle::new(Context {
                patterns: vec![Pattern {
                    tracks: vec![],
                    choke_grps: vec![],
                    name: "Pattern 1".to_string(),
                    division: Division::E,
                }],
                pattern_id: 0,
                queued_pattern_id: 0,
                saved_patterns: vec![],
                sample_files: vec![],
                default_len: 8,
                tempo: 120,
                // corresponds to 120 bpm
                pulse_interval: Duration::from_secs_f32(2.5/120.0),
                playing: false,
                command_rx_ch: command_rx,
                last_cmd: Command::Unspecified,
                midi_conn: None,
                stream,
                state_tx_ch: vec![]
            }),
            latency: Duration::ZERO,
            sleep_interval: Duration::from_secs_f32(1.0/24.0),
            // pulses per bar, 24 per quarter note
            // afaik this is the rate to send midi clock signals
            ppb: 24*4,
            pulse_idx: 0,
            command_tx_ch: command_tx,
            sleeper: spin_sleep::SpinSleeper::new(1_012_550_000).with_spin_strategy(spin_sleep::SpinStrategy::SpinLoopHint)
        };
        s.ctx.with_lock(|ctx| {
            if let Err(e) = ctx.refresh_saved_patterns() {
                println!("Failed to refresh saved patterns: {}", e);
            }
            if let Err(e) = ctx.refresh_sample_files() {
                println!("Failed to refresh sample files: {}", e);
            }
        });
        s
    }

    /// Sets tempo via ctx handle
    pub fn set_tempo(&mut self, bpm: u8) {
        self.ctx.set_tempo(bpm);
    }

    pub fn play(&mut self) {
        self.ctx.enable_play();
    }

    pub fn stop(&mut self) {
        self.ctx.disable_play();
    }

    // Starts an active midi connection to the specified port
    /// 
    /// I've not yet quite figured out how to share MidiOutput so I'm just
    /// persisting the connection, which should accomplish what we need
    pub fn connect_midi(&mut self, port: MidiOutputPort) -> Result<(), Box<dyn Error>> {
        let midi_output = MidiOutput::new("Sequencer")?;
        let conn = midi_output.connect(&port, "Sequencer")?;
        self.ctx.with_lock(|ctx| {
            ctx.midi_conn = Some(Arc::new(conn));
        });
        Ok(())
    }

    /// Adds an empty track to the sequencer at the current pattern
    /// 
    /// Because trks are wrapped in a property handler to ensure thread safety, we can't directly return access
    /// to the track. Instead the index of the created track is returned for reference
    /// This index serves as the track Id and is referred to as such throughout the code
    /// So be aware track_id is its location in the tracks list, while track_idx is the current
    /// playhead position of the track's slots.
    pub fn add_track(&mut self, sample_path: String) -> Result<TrackHandle, Box<dyn Error>> {
        self.ctx.with_lock(|ctx| {
            ctx.patterns[ctx.pattern_id].add_track(ctx.stream.clone(), ctx.default_len, sample_path)?;
            Ok(TrackHandle::new(self.ctx.clone(), ctx.patterns[ctx.pattern_id].tracks.len() as u8 - 1))
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

    /// The VIP function. Plays tracks, sends state, updates latency
    fn play_next(&mut self) {
        let playing = self.ctx.with_lock(|ctx| { ctx.playing });
        if playing {
            let start = Instant::now();
            // If pattern is queued, we switch to it on the 0 to maintain
            // the expected beat (this is similar to default Ableton behavior
            // in session mode for instance)
            self.ctx.with_lock(|ctx| {
                if self.pulse_idx == 0 {
                    if ctx.queued_pattern_id != ctx.pattern_id {
                        ctx.pattern_id = ctx.queued_pattern_id;
                        ctx.reset_playheads();
                    }
                }

                // hmm might have to create a spare vec of pulses where 1 is trigger to handle swing patterns
                // and then in fact we might have to move that tracking to the track
                let pattern = &mut ctx.patterns[ctx.pattern_id];
                if self.pulse_idx % (self.ppb / pattern.division as u8) == 0 {
                    let mut triggered_ids: Vec<usize> = vec![];
                    let tracks = &mut pattern.tracks;
                    for (i, t) in tracks.into_iter().enumerate() {
                        let vel = &mut t.slots[t.idx].velocity;
                        if *vel > 0 {
                            Sequencer::append_sample_to_sink(t.sink.clone(), t.sample.clone(), vel);
                            triggered_ids.push(i);
                        }

                        t.idx = (t.idx + 1) % t.len;
                    }
                    
                    // Redefine as immutable to prevent triggering borrow checker
                    let pattern = &ctx.patterns[ctx.pattern_id];
                    let tracks = &pattern.tracks;
                    for i in 0..tracks.len() {
                        if pattern.is_trk_choked(&triggered_ids, i) {
                            tracks[i].sink.skip_one();
                        }
                    }
                }

                // if the ppb cycle has reset, send a start signal
                // to sync devices (clock is just for tempo)
                if let Some(midi_conn) = &mut ctx.midi_conn {
                    let conn = Arc::<MidiOutputConnection>::get_mut(midi_conn).unwrap();
                    // if self.pulse_idx % self.ppb == 0 {
                    //     // start
                    //     conn.send(&[0xFA]).unwrap();
                    // }
                    // clock
                    conn.send(&[0xF8]).unwrap();
                }
            });
            self.pulse_idx = (self.pulse_idx + 1) % self.ppb;

            self.set_latency(Instant::now().duration_since(start));

        } else if self.pulse_idx != 0 {
            self.pulse_idx = 0;
            self.ctx.with_lock(|ctx| {
                ctx.patterns[ctx.pattern_id].reset_playheads();
            });
        }

        self.tx_state();
    }

    /// Attempts to keep timing tight by subtracting processing time from overall wait between beats
    fn set_latency(&mut self, t: Duration) {
        self.latency = Duration::from_nanos(((self.latency + t).as_nanos() / 2) as u64);
        self.ctx.with_lock(|ctx| {
            self.sleep_interval = ctx.pulse_interval - ctx.pulse_interval.min(self.latency)
        })
    }

    /// Uses ctx handle to set time division (4/4 time is quarter division, 4/8 is eighth, etc)
    pub fn set_division(&mut self, division: Division) {
        self.ctx.with_lock(|ctx| {
            ctx.patterns[ctx.pattern_id].division = division;
        });
    }

    /// Creates a new channel to send state updates to
    pub fn get_state_rx(&mut self) -> mpsc::Receiver<StateUpdate> {
        let (tx, rx) = mpsc::channel();
        self.ctx.with_lock(|ctx| {
            ctx.state_tx_ch.push(tx);
        });
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
        self.ctx.with_lock(|ctx| {
            let trks: Vec<TrackState> = ctx
                .patterns[ctx.pattern_id]
                .tracks
                .iter()
                .map(|t| {
                    TrackState {
                        slots: t.slots.iter().map(|s| { s.velocity }).collect(),
                        name: t.name.clone(),
                        idx: t.idx,
                        len: t.len,
                        sample_path: t.sample_path.clone(),
                    }
                })
                .collect();

            for tx in &ctx.state_tx_ch {
                let _ = tx.send(StateUpdate::SeqState(SeqState {
                    tempo: ctx.tempo,
                    trks: trks.clone(),
                    division: ctx.patterns[ctx.pattern_id].division as u8,
                    default_len: ctx.default_len,
                    latency: self.latency,
                    last_cmd: ctx.last_cmd.clone(),
                    playing: ctx.playing,
                    pattern_id: ctx.pattern_id,
                    pattern_len: ctx.patterns.len(),
                    pattern_name: ctx.patterns[ctx.pattern_id].name.clone(),
                    queued_pattern_id: ctx.queued_pattern_id,
                }));
            }
        })
    }

    /// Receives commands and modifies sequencer state accordingly
    /// 
    /// You can run this in its own thread. It does not own the sequencer
    /// instance hence we use a ctx handle to modify the sequencer state
    /// There's a slight weirdness with this paradigm in that one shot
    /// sample playing will directly add to the track playback sink, instead
    /// of modifying a property. Maybe tracks are not fully definable as properties
    /// but we gain functionality treating them as such
    pub fn run_command_loop(ctx: ContextHandle) {
        loop {
            ctx.with_lock(|ctx| {
                if let Ok(cmd) = ctx.command_rx_ch.try_recv() {
                    ctx.last_cmd = cmd.clone();
                    match cmd {
                        Command::SetTempo(bpm) => ctx.set_tempo(bpm),
                        Command::PlaySound(trk_id, vel) => (|trk_id, vel| {
                                let trk: &mut Track = &mut ctx.patterns[ctx.pattern_id].tracks[trk_id];
                                let mut vel = vel;
                                let v = &mut vel;
                                Sequencer::append_sample_to_sink(trk.sink.clone(), trk.sample.clone(), v);
                                let trks = &ctx.patterns[ctx.pattern_id].tracks;
                                for i in 0..trks.len() {
                                    if ctx.patterns[ctx.pattern_id].is_trk_choked(&vec![trk_id], i) {
                                        trks[i].sink.skip_one();
                                    }
                                }
                            })(trk_id, vel),
                        Command::PlaySequencer => ctx.enable_play(),
                        Command::StopSequencer => ctx.disable_play(),
                        Command::SetDivision(div) => ctx.patterns[ctx.pattern_id].division = div,
                        Command::SetSlotVelocity(trk, slot, vel) => {
                            ctx.patterns[ctx.pattern_id].tracks[trk].slots[slot].velocity = vel;
                        },
                        // Adding a new pattern will duplicate the current pattern
                        // tracks and clear the slots
                        Command::AddPattern => {
                            let new_id = ctx.patterns.len();
                            ctx.patterns.push(ctx.patterns[ctx.pattern_id].clone());
                            ctx.patterns[new_id].zero_all_tracks();
                            ctx.patterns[new_id].name = format!("Pattern {}", new_id + 1);
                            if ctx.playing {
                                ctx.queued_pattern_id = new_id;
                            } else {
                                ctx.pattern_id = new_id;
                            }
                        },
                        Command::RemovePattern(idx) => {
                            ctx.patterns.remove(idx);
                            if idx < ctx.pattern_id {
                                ctx.pattern_id -= 1;
                            }
                        },
                        Command::SelectPattern(idx) => {
                            if !ctx.playing {
                                ctx.pattern_id = idx;
                            } else {
                                ctx.queued_pattern_id = idx;
                            }
                        },
                        Command::SetPatternLength(len) => {
                            ctx.patterns[ctx.pattern_id].set_len(len);
                        },
                        Command::SavePattern => {
                            if let Err(e) = ctx.save_pattern() {
                                println!("Failed to save pattern: {}", e);
                            }
                        },
                        Command::LoadPattern(pattern_fname) => {
                            if let Err(e) = ctx.load_pattern(pattern_fname.clone()) {
                                println!("Failed to load pattern: {}", e);
                            }
                        },
                        Command::ListPatterns => {
                            ctx.send_file_state(FileType::Pattern);
                        },
                        Command::ListSamples => {
                            ctx.send_file_state(FileType::Sample);
                        },
                        Command::AddTrack(sample_path) => {
                            ctx.patterns[ctx.pattern_id].add_track(ctx.stream.clone(), ctx.default_len, sample_path).unwrap();
                        },
                        Command::SetTrackSample(trk_id, sample_path) => {
                            ctx.patterns[ctx.pattern_id].set_track_sample(trk_id, sample_path).unwrap();
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
        self.sleeper.sleep(self.sleep_interval);
    }

    /// Runs the sequencer
    pub fn run_sound_loop(mut seq: Self) {
        loop {
            seq.play_next();
            seq.sleep();
            // yield_now();
        }
    }

}