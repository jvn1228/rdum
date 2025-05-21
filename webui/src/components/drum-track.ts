import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { Track } from '../models/types';
import './drum-pad';

@customElement('drum-track')
export class DrumTrack extends LitElement {
  @property({ type: Object }) track!: Track;
  @property({ type: Number }) currentStep = -1;

  static styles = css`
    :host {
      display: block;
    }

    .track-row {
      display: flex;
      flex-direction: row;
      align-items: center;
      margin-bottom: 12px;
    }

    .track-label {
      width: 100px;
      font-weight: 500;
      margin-right: 12px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .pads-container {
      display: grid;
      grid-template-columns: repeat(16, 1fr);
      grid-gap: 8px;
      flex: 1;
    }

    @media (max-width: 768px) {
      .pads-container {
        grid-template-columns: repeat(8, 1fr);
      }
    }
  `;

  render() {
    return html`
      <div class="track-row">
        <div class="track-label">${this.track.name}</div>
        <div class="pads-container">
          ${this.track.slots.map((active, index) => html`
            <drum-pad 
              ?active=${active} 
              ?trigger=${this.currentStep === index}
              @pad-toggled=${(e: CustomEvent) => this._handlePadToggled(index, e.detail.value)}
            ></drum-pad>
          `)}
        </div>
      </div>
    `;
  }

  _handlePadToggled(index: number, value: boolean) {
    this.dispatchEvent(new CustomEvent('track-pad-toggled', {
      detail: {
        trackId: this.track.id,
        slotIndex: index,
        value
      },
      bubbles: true,
      composed: true
    }));
  }

  updated(changedProperties: Map<string, any>) {
    if (changedProperties.has('currentStep') && this.currentStep >= 0) {
      const padElements = this.shadowRoot?.querySelectorAll('drum-pad');
      if (padElements && padElements[this.currentStep]) {
        (padElements[this.currentStep] as any).triggerAnimation();
      }
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'drum-track': DrumTrack;
  }
}
