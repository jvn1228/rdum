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
}

export enum MessageType {
  STATE_UPDATE = 'state_update',
  TOGGLE_PAD = 'toggle_pad',
  CHANGE_PATTERN = 'change_pattern',
  PLAY = 'play',
  STOP = 'stop',
  CHANGE_TEMPO = 'change_tempo',
}

export interface WebSocketMessage {
  type: MessageType;
  payload: any;
}

export interface TogglePadPayload {
  patternId: number;
  trackId: number;
  slotIndex: number;
  value: boolean;
}

export interface ChangePatternPayload {
  patternId: number;
}

export interface ChangeTempoPayload {
  tempo: number;
}
