export interface Track {
  name: string;
  slots: number[];
  idx: number;
  len: number;
  sample_path: string;
}

export interface Pattern {
  id: number;
  name: string;
  tracks: Track[];
}

export interface DrumMachineState {
  trks: Track[];
  playing: boolean;
  tempo: number;
  division: number; // Added to match Rust struct
  default_len: number; // Added to match Rust struct
  latency?: any; // Added to match Rust struct, using any type for Duration
  last_cmd?: any; // Added to match Rust struct, using any type for Command
  pattern_len: number;
  pattern_id: number;
  pattern_name: string;
  queued_pattern_id: number;
  swing: number;
}

export enum FileType {
  PATTERN = 'pattern',
  SAMPLE = 'sample',
}

export interface FileStateMsg {
  type: FileType;
  files: string[];
}

export interface FileState {
  patterns: string[];
  samples: string[];
}

export enum MessageType {
  STATE_UPDATE = 'state_update',
  FILE_STATE_UPDATE = 'file_state_update',
  SET_SLOT_VELOCITY = 'set_slot_velocity',
  CHANGE_PATTERN = 'change_pattern',
  PLAY_SEQUENCER = 'play_sequencer',
  STOP_SEQUENCER = 'stop_sequencer',
  SET_TEMPO = 'set_tempo',
  ADD_PATTERN = 'add_pattern',
  REMOVE_PATTERN = 'remove_pattern',
  SELECT_PATTERN = 'select_pattern',
  SET_PATTERN_LENGTH = 'set_pattern_length',
  SET_DIVISION = 'set_division',
  SAVE_PATTERN = 'save_pattern',
  LOAD_PATTERN = 'load_pattern',
  LIST_PATTERNS = 'list_patterns',
  LIST_SAMPLES = 'list_samples',
  SET_TRACK_SAMPLE = 'set_track_sample',
  ADD_TRACK = 'add_track',
  SET_SWING = 'set_swing',
}

export interface WebSocketMessage {
  type: MessageType;
  payload: any;
}

export interface TogglePadPayload {
  patternId: number;
  trackId: number;
  slotIdx: number;
  velocity: number;
}

export interface ChangePatternPayload {
  patternId: number;
}

export interface ChangeTempoPayload {
  tempo: number;
}

export interface SelectPatternPayload {
  patternId: number;
}

export interface SetPatternLengthPayload {
  length: number;
}

export interface SetDivisionPayload {
  division: number;
}

export interface LoadPatternPayload {
  fname: string;
}

export interface SetTrackSamplePayload {
  trackId: number;
  samplePath: string;
}

export interface SetSwingPayload {
  swing: number;
}
