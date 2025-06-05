import { LitElement, html, css } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import '@material/web/labs/navigationbar/navigation-bar.js';
import '@material/web/labs/navigationtab/navigation-tab.js';
import '@material/web/icon/icon.js';
import '@material/web/textfield/filled-text-field.js';
import '@material/web/select/filled-select.js';
import '@material/web/select/select-option.js';
import './drum-track';
import './pattern-selector';
import './transport-controls';
import './theme-switch';
import './machine-info';
import { WebSocketService } from '../services/websocket-service';
import { DrumMachineState } from '../models/types';

@customElement('drum-machine-app')
export class DrumMachineApp extends LitElement {
  // Fixed default state to match Rust backend's State struct
  @state() private drumState: DrumMachineState = {
    trks: [],
    playing: false,
    tempo: 120,
    division: 16,  // Default to 16th notes
    pattern_len: 1,
    pattern_id: 0,
    pattern_name: "Pattern 1",
    latency: 0,
    default_len: 16,
    queued_pattern_id: 0,
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

    .pattern-length-controls {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 3rem;
      margin-left: 16px;
    }

    .pattern-length-controls label {
      font-size: 0.9em;
      color: var(--text-secondary-color);
    }

    md-select-option, md-filled-select {
      min-width: 7rem;
      max-width: 7rem;
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
              .patternName=${currentPattern}
              .patternLen=${this.drumState.pattern_len}
              .currentPatternId=${this.drumState.pattern_id}
              .queuedPatternId=${this.drumState.queued_pattern_id}
              @pattern-selected=${this.handlePatternSelected}
              @add-pattern=${this.handleAddPattern}
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
            <div class="tracks-title">Pattern: ${currentPattern || 'None'}</div>
            <div class="pattern-length-controls">
              <label for="patternLengthInput">Steps:</label>
              <md-filled-text-field
                id="patternLengthInput"
                type="number"
                value=${this.drumState.trks[0]?.len || this.drumState.default_len}
                min="1"
                max="256" 
                @change=${this._handlePatternLengthChange}
              ></md-filled-text-field>
              <label for="divisionInput">Division:</label>
              <md-filled-select
                id="divisionInput"
                value=${this.drumState.division}
                @change=${this._handleDivisionChange}
              >
                <md-select-option value="1">1</md-select-option>
                <md-select-option value="2">1/2</md-select-option>
                <md-select-option value="3">1/4.</md-select-option>
                <md-select-option value="4">1/4</md-select-option>
                <md-select-option value="6">1/8.</md-select-option>
                <md-select-option value="8">1/8</md-select-option>
                <md-select-option value="12">1/16.</md-select-option>
                <md-select-option value="16">1/16</md-select-option>
                <md-select-option value="24">1/32.</md-select-option>
                <md-select-option value="32">1/32</md-select-option>
              </md-filled-select>
            </div>
            <div class="drum-grid-container">
              ${this.drumState.trks.map((track, idx) => html`
                <drum-track
                  .track=${track}
                  .trkId=${idx}
                  @track-pad-toggled=${this.handlePadToggled}
                ></drum-track>
              `)}
            </div>
          </div>
        </div>
      </div>
    `;
  }

  getCurrentPattern(): string | undefined {
    return this.drumState.pattern_name;
  }

  handleStateUpdate(state: DrumMachineState) {
    this.drumState = { ...state };
  }

  handlePatternSelected(e: CustomEvent) {
    const { patternId } = e.detail;
    this.webSocketService.selectPattern(patternId);
  }

  handleAddPattern() {
    this.webSocketService.addPattern();
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
    const { trackId, slotIdx, velocity } = e.detail;
    this.webSocketService.togglePad(
      this.currentPatternId, // Use the fixed pattern ID
      trackId,
      slotIdx,
      velocity
    );
  }

  private _handlePatternLengthChange(e: Event) {
    const input = e.target as HTMLInputElement; // Or more specifically MdFilledTextField if its type is available
    let newLength = parseInt(input.value, 10);

    if (isNaN(newLength) || newLength < 1) {
      console.warn("Invalid pattern length, must be at least 1:", input.value);
      newLength = Math.max(1, this.drumState.trks[0]?.len || this.drumState.default_len); // Revert to current or default
      input.value = newLength.toString(); // Update input field
      return;
    }
    
    // Optional: Cap the maximum length
    if (newLength > 256) { 
      console.warn("Pattern length capped at 256:", input.value);
      newLength = 256;
      input.value = newLength.toString(); // Update input field
    }

    this.webSocketService.setPatternLength(newLength);
  }

  private _handleDivisionChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    const newDivision = parseInt(select.value, 10);
    this.webSocketService.setDivision(newDivision);
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
