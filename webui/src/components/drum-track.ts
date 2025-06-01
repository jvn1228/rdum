import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { Track } from '../models/types';
import './drum-pad';

@customElement('drum-track')
export class DrumTrack extends LitElement {
  @property({ type: Object }) track!: Track;
  @property({ type: Number }) trkIdx = -1;

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
          ${this.track.slots.map((vel, index) => {
            let idx = (index + 1) % this.track.slots.length;
            return html`
              <drum-pad 
                vel=${vel}
                ?trigger=${this.track.idx === idx}
                @pad-toggled=${(e: CustomEvent) => this._handlePadToggled(index, e.detail.value)}
              ></drum-pad>`
          })}
        </div>
      </div>
    `;
  }

  _handlePadToggled(index: number, value: boolean) {
    this.dispatchEvent(new CustomEvent('track-pad-toggled', {
      detail: {
        trackIdx: this.trkIdx,
        slotIdx: index,
        velocity: value ? 127 : 0
      },
      bubbles: true,
      composed: true
    }));
  }

  // updated(changedProperties: Map<string, any>) {
  //   if (changedProperties.has('trkIdx') && this.trkIdx >= 0) {
  //     const padElements = this.shadowRoot?.querySelectorAll('drum-pad');
  //     let idx = (this.trkIdx + this.track.slots.length - 1) % this.track.slots.length;
  //     if (padElements && padElements[idx]) {
  //       (padElements[idx] as any).triggerAnimation();
  //     }
  //   }
  // }
}

declare global {
  interface HTMLElementTagNameMap {
    'drum-track': DrumTrack;
  }
}
