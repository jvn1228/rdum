import * as types from '../models/types';

export class WebSocketService {
  private socket: WebSocket | null = null;
  private url: string;
  private reconnectDelay: number = 1000;
  private listeners: ((state: types.DrumMachineState) => void)[] = [];

  constructor(url?: string) {
    if (url) {
      this.url = url;
    } else {
      // Default to using the current hostname the page is served from,
      // assuming the WebSocket server is on the same host, port 8080.
      const hostname = window.location.hostname;
      this.url = `ws://${hostname}:8080`;
    }
    console.log(`WebSocketService attempting to connect to: ${this.url}`);
  }

  public connect(): void {
    this.socket = new WebSocket(this.url);

    this.socket.addEventListener('open', () => {
      console.log('WebSocket connection established');
      this.reconnectDelay = 1000; // Reset reconnect delay on successful connection
    });

    this.socket.addEventListener('message', (event) => {
      try {
        const data = JSON.parse(event.data);
        
        // Case 1: The message is a formatted WebSocketMessage with type and payload
        if (data.type && data.type === types.MessageType.STATE_UPDATE && data.payload) {
          const state = data.payload as types.DrumMachineState;
          this.notifyListeners(state);
          return;
        }
        
        // Case 2: The message is a welcome message or other non-state message with a type field
        if (data.type === 'connection') {
          console.log('Connection status:', data.status);
          return;
        }
        
        // Case 3: The message is a direct state object from the Rust backend
        // Check if it has the expected state properties
        if (data.patterns !== undefined && 
            data.currentPatternId !== undefined && 
            data.isPlaying !== undefined && 
            data.currentStep !== undefined && 
            data.tempo !== undefined) {
          const state = data as types.DrumMachineState;
          console.log('Received state update:', state);
          this.notifyListeners(state);
          return;
        }
        
        console.log('Received unhandled message format:', data);
      } catch (error) {
        console.error('Error parsing WebSocket message:', error);
      }
    });

    this.socket.addEventListener('close', () => {
      console.log('WebSocket connection closed. Attempting to reconnect...');
      setTimeout(() => {
        this.reconnectDelay = Math.min(this.reconnectDelay * 1.5, 30000); // Exponential backoff
        this.connect();
      }, this.reconnectDelay);
    });

    this.socket.addEventListener('error', (error) => {
      console.error('WebSocket error:', error);
    });
  }

  public disconnect(): void {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }

  public togglePad(patternId: number, trackId: number, slotIdx: number, velocity: number): void {
    const payload: types.TogglePadPayload = {
      patternId,
      trackId,
      slotIdx,
      velocity
    };
    this.sendMessage(types.MessageType.SET_SLOT_VELOCITY, payload);
  }

  public selectPattern(patternId: number): void {
    const payload: types.SelectPatternPayload = {
      patternId
    };
    this.sendMessage(types.MessageType.SELECT_PATTERN, payload);
  }

  public play(): void {
    this.sendMessage(types.MessageType.PLAY_SEQUENCER, {});
  }

  public stop(): void {
    this.sendMessage(types.MessageType.STOP_SEQUENCER, {});
  }

  public changeTempo(tempo: number): void {
    const payload: types.ChangeTempoPayload = {
      tempo
    };
    this.sendMessage(types.MessageType.SET_TEMPO, payload);
  }

  public addPattern(): void {
    this.sendMessage(types.MessageType.ADD_PATTERN, {});
  }

  public setPatternLength(length: number): void {
    const payload: types.SetPatternLengthPayload = {
      length
    };
    this.sendMessage(types.MessageType.SET_PATTERN_LENGTH, payload);
  }

  public setDivision(division: number): void {
    const payload: types.SetDivisionPayload = {
      division
    };
    this.sendMessage(types.MessageType.SET_DIVISION, payload);
  }

  private sendMessage(type: types.MessageType, payload: any): void {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      const message: types.WebSocketMessage = {
        type,
        payload
      };
      this.socket.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket not connected. Message not sent:', type, payload);
    }
  }

  public addStateListener(listener: (state: types.DrumMachineState) => void): void {
    this.listeners.push(listener);
  }

  public removeStateListener(listener: (state: types.DrumMachineState) => void): void {
    this.listeners = this.listeners.filter(l => l !== listener);
  }

  private notifyListeners(state: types.DrumMachineState): void {
    this.listeners.forEach(listener => listener(state));
  }
}
