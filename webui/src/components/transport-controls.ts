import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/icon/icon.js';
import '@material/web/slider/slider.js';


@customElement('transport-controls')
export class TransportControls extends LitElement {
  @property({ type: Boolean }) isPlaying = false;
  @property({ type: Number }) tempo = 120;

  static styles = css`
    :host {
      display: block;
    }

    .transport-controls {
      display: flex;
      flex-direction: column;
      gap: 16px;
      margin: 16px 0;
      padding: 16px;
      background-color: var(--surface-color, #ffffff);
      border-radius: 8px;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    }

    .play-controls {
      display: flex;
      gap: 12px;
    }

    .tempo-control {
      display: flex;
      align-items: center;
      gap: 16px;
      margin-top: 16px;
    }

    .tempo-value {
      min-width: 60px;
      text-align: center;
      font-size: 18px;
    }

    .section-title {
      font-size: 18px;
      font-weight: 500;
      margin-bottom: 16px;
    }

    mwc-slider {
      flex: 1;
    }

    md-filled-button {
      min-width: 100px;
    }
    
    md-icon {
      font-size: 24px;
      margin-right: 8px;
      display: inline-flex;
      vertical-align: middle;
    }
  `;

  render() {
    return html`
      <div class="transport-controls">
        <div class="section-title">Transport</div>
        
        <div class="play-controls">
          <md-filled-button 
            ?disabled=${this.isPlaying}
            @click=${this._handlePlay}
          >
            <md-icon slot="icon">play_arrow</md-icon>
            <span>Play</span>
          </md-filled-button>
          
          <md-filled-button 
            ?disabled=${!this.isPlaying}
            @click=${this._handleStop}
          >
            <md-icon slot="icon">stop</md-icon>
            <span>Stop</span>
          </md-filled-button>
        </div>
        
        <div class="tempo-control">
          <span>Tempo:</span>
          <md-slider
            min="60"
            max="200"
            value=${this.tempo}
            @change=${this._handleTempoChange}
            labeled
          ></md-slider>
          <div class="tempo-value">${this.tempo} BPM</div>
        </div>
      </div>
    `;
  }

  _handlePlay() {
    this.dispatchEvent(new CustomEvent('play', {
      bubbles: true,
      composed: true
    }));
  }

  _handleStop() {
    this.dispatchEvent(new CustomEvent('stop', {
      bubbles: true,
      composed: true
    }));
  }

  _handleTempoChange(e: CustomEvent) {
    const newTempo = Math.round(e.detail.value);
    this.dispatchEvent(new CustomEvent('tempo-change', {
      detail: { tempo: newTempo },
      bubbles: true,
      composed: true
    }));
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'transport-controls': TransportControls;
  }
}
