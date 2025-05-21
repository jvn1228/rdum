export interface Track {
  id: number;
  name: string;
  slots: boolean[];
}

export interface Pattern {
  id: number;
  name: string;
  tracks: Track[];
}

export interface DrumMachineState {
  patterns: Pattern[];
  currentPatternId: number;
  isPlaying: boolean;
  currentStep: number;
  tempo: number;
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
