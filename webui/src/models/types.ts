export interface Track {
  name: string;
  slots: number[];
  idx: number;
  len: number;
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
  pattern_idx: number;
  pattern_name: string;
}

export enum MessageType {
  STATE_UPDATE = 'state_update',
  SET_SLOT_VELOCITY = 'set_slot_velocity',
  CHANGE_PATTERN = 'change_pattern',
  PLAY_SEQUENCER = 'play_sequencer',
  STOP_SEQUENCER = 'stop_sequencer',
  SET_TEMPO = 'set_tempo',
}

export interface WebSocketMessage {
  type: MessageType;
  payload: any;
}

export interface TogglePadPayload {
  patternIdx: number;
  trackIdx: number;
  slotIdx: number;
  velocity: number;
}

export interface ChangePatternPayload {
  patternId: number;
}

export interface ChangeTempoPayload {
  tempo: number;
}
