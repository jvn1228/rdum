import { LitElement, html, css } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import '@material/web/labs/navigationbar/navigation-bar.js';
import '@material/web/labs/navigationtab/navigation-tab.js';
import '@material/web/icon/icon.js';
import './drum-track';
import './pattern-selector';
import './transport-controls';
import './theme-switch';
import './machine-info';
import { WebSocketService } from '../services/websocket-service';
import { DrumMachineState, Pattern } from '../models/types';

@customElement('drum-machine-app')
export class DrumMachineApp extends LitElement {
  // Fixed default state to match Rust backend's State struct
  @state() private drumState: DrumMachineState = {
    trks: [],
    playing: false,
    trk_idx: 0,
    tempo: 120,
    division: 16,  // Default to 16th notes
    len: 16       // Default pattern length
  };

  // Since the backend doesn't use pattern IDs, we'll use a fixed value
  private currentPatternId: number = 1;

  private webSocketService: WebSocketService;

  constructor() {
    super();
    this.webSocketService = new WebSocketService();
    this.webSocketService.addStateListener(this.handleStateUpdate.bind(this));
  }

  static styles = css`
    :host {
      display: block;
    }

    .drum-machine-container {
      display: flex;
      flex-direction: column;
      width: 100%;
      min-height: 100vh;
    }
    
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 16px;
      background-color: var(--primary-color);
      color: var(--text-primary-color);
      height: 64px;
      box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
      z-index: 11;
      position: relative;
    }

    .content {
      flex: 1;
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 24px;
    }

    .pattern-selector-container {
      padding: 16px;
    }

    .transport-container {
      padding: 16px;
    }

    .tracks-container {
      padding: 24px;
    }

    .tracks-title {
      font-size: 20px;
      font-weight: 500;
      margin-bottom: 20px;
      color: var(--text-primary-color);
    }

    md-navigation-bar {
      --md-navigation-bar-container-color: var(--primary-color);
      --md-navigation-bar-label-text-color: var(--text-primary-color);
      --md-navigation-bar-icon-color: var(--text-primary-color);
      box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
      z-index: 10;
      position: relative;
    }
    
    .app-title {
      font-size: 20px;
      font-weight: 500;
      margin: 0;
      padding: 16px;
      color: var(--text-primary-color);
    }

    .app-bar-actions {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .drum-grid-container {
      transition: background-color 0.3s ease;
    }
  `;

  connectedCallback() {
    super.connectedCallback();
    this.webSocketService.connect();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this.webSocketService.disconnect();
  }

  render() {
    const currentPattern = this.getCurrentPattern();
    
    return html`
      <div class="drum-machine-container">
        <header>
          <div class="app-title">RDUM - Drum Machine</div>
          <div class="app-bar-actions">
            <theme-switch @theme-changed=${this._handleThemeChanged}></theme-switch>
          </div>
        </header>
        

        
        <div class="content">
          <div class="glass-card">
            <machine-info .state=${this.drumState}></machine-info>
          </div>
          <div class="pattern-selector-container glass-card">
            <pattern-selector
              .patterns={[{ id: 1, name: 'Pattern 1', tracks: [] }]}
              .currentPatternId=${this.currentPatternId}
              @pattern-selected=${this.handlePatternSelected}
            ></pattern-selector>
          </div>
          <div class="transport-container glass-card">
            <transport-controls
              .isPlaying=${this.drumState.playing}
              .tempo=${this.drumState.tempo}
              @play=${this.handlePlay}
              @stop=${this.handleStop}
              @tempo-change=${this.handleTempoChange}
            ></transport-controls>
          </div>
          
          <div class="tracks-container glass-card">
            <div class="tracks-title">Pattern: ${currentPattern?.name || 'None'}</div>
            <div class="drum-grid-container">
              ${currentPattern?.tracks.map(track => html`
                <drum-track
                  .track=${track}
                  .trkIdx=${this.drumState.trk_idx}
                  @track-pad-toggled=${this.handlePadToggled}
                ></drum-track>
              `)}
            </div>
          </div>
        </div>
      </div>
    `;
  }

  getCurrentPattern(): Pattern | undefined {
    return {
      id: 1,
      name: "Pattern 1",
      tracks: this.drumState.trks
    }
    // return this.drumState.patterns.find(p => p.id === this.drumState.currentPatternId);
  }

  handleStateUpdate(state: DrumMachineState) {
    this.drumState = { ...state };
  }

  handlePatternSelected(e: CustomEvent) {
    const { patternId } = e.detail;
    this.webSocketService.changePattern(patternId);
  }

  handlePlay() {
    this.webSocketService.play();
  }

  handleStop() {
    this.webSocketService.stop();
  }

  handleTempoChange(e: CustomEvent) {
    const { tempo } = e.detail;
    this.webSocketService.changeTempo(tempo);
  }

  handlePadToggled(e: CustomEvent) {
    const { trackId, slotIndex, value } = e.detail;
    this.webSocketService.togglePad(
      this.currentPatternId, // Use the fixed pattern ID
      trackId,
      slotIndex,
      value
    );
  }
  
  _handleThemeChanged(e: CustomEvent) {
    const { theme } = e.detail;
    console.log(`Theme changed to: ${theme}`);
    // You could persist this preference to the server if needed
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'drum-machine-app': DrumMachineApp;
  }
}
