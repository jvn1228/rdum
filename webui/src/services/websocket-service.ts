import { MessageType, WebSocketMessage, TogglePadPayload, ChangePatternPayload, ChangeTempoPayload, DrumMachineState } from '../models/types';

export class WebSocketService {
  private socket: WebSocket | null = null;
  private url: string;
  private reconnectDelay: number = 1000;
  private listeners: ((state: DrumMachineState) => void)[] = [];

  constructor(url: string = 'ws://localhost:8080') {
    this.url = url;
  }

  public connect(): void {
    this.socket = new WebSocket(this.url);

    this.socket.addEventListener('open', () => {
      console.log('WebSocket connection established');
      this.reconnectDelay = 1000; // Reset reconnect delay on successful connection
    });

    this.socket.addEventListener('message', (event) => {
      try {
        const message = JSON.parse(event.data) as WebSocketMessage;
        
        if (message.type === MessageType.STATE_UPDATE) {
          const state = message.payload as DrumMachineState;
          this.notifyListeners(state);
        }
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

  public togglePad(patternId: number, trackId: number, slotIndex: number, value: boolean): void {
    const payload: TogglePadPayload = {
      patternId,
      trackId,
      slotIndex,
      value
    };
    this.sendMessage(MessageType.TOGGLE_PAD, payload);
  }

  public changePattern(patternId: number): void {
    const payload: ChangePatternPayload = {
      patternId
    };
    this.sendMessage(MessageType.CHANGE_PATTERN, payload);
  }

  public play(): void {
    this.sendMessage(MessageType.PLAY, {});
  }

  public stop(): void {
    this.sendMessage(MessageType.STOP, {});
  }

  public changeTempo(tempo: number): void {
    const payload: ChangeTempoPayload = {
      tempo
    };
    this.sendMessage(MessageType.CHANGE_TEMPO, payload);
  }

  private sendMessage(type: MessageType, payload: any): void {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      const message: WebSocketMessage = {
        type,
        payload
      };
      this.socket.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket not connected. Message not sent:', type, payload);
    }
  }

  public addStateListener(listener: (state: DrumMachineState) => void): void {
    this.listeners.push(listener);
  }

  public removeStateListener(listener: (state: DrumMachineState) => void): void {
    this.listeners = this.listeners.filter(l => l !== listener);
  }

  private notifyListeners(state: DrumMachineState): void {
    this.listeners.forEach(listener => listener(state));
  }
}
