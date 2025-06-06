import { LitElement, html, css } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';
import { Track } from '../models/types';
import './drum-pad';

@customElement('drum-track')
export class DrumTrack extends LitElement {
  @property({ type: Object }) track!: Track;
  @property({ type: Number }) trkId = -1;
  @property({ type: Array }) samples!: string[];

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

    .sample-select {
      max-width: 12rem;
      min-width: 8rem;
      margin-right: 2rem;
      padding: 0.5rem 2rem 0.5rem 0.75rem;
      border-radius: 4px;
      border: 1px solid var(--md-sys-color-outline);
      background-color: var(--md-sys-color-surface-container-highest);
      color: var(--md-sys-color-on-surface);
      font-size: 0.875rem;
      line-height: 1.5;
      appearance: none;
      background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='%23666'%3E%3Cpath d='M7 10l5 5 5-5z'/%3E%3C/svg%3E");
      background-repeat: no-repeat;
      background-position: right 0.5rem center;
      background-size: 1.5rem;
      transition: border-color 0.2s, box-shadow 0.2s;
    }

    .sample-select:focus {
      outline: none;
      border-color: var(--md-sys-color-primary);
      box-shadow: 0 0 0 2px rgba(var(--md-sys-color-primary-rgb, 0, 0, 0), 0.1);
    }
    
    .sample-select:hover {
      border-color: var(--md-sys-color-on-surface-variant);
    }
  `;

  @state() private selectedSample: string = '';

  updated(changedProperties: Map<string, any>) {
    if (changedProperties.has('track')) {
      this.selectedSample = this.track.sample_path || '';
    }
  }

  render() {
    return html`
      <div class="track-row">
        <select
          id="sampleSelect"
          class="sample-select"
          value=${this.selectedSample}
          @change=${this._handleSampleChange}
        >
          ${this.samples.map(sample => 
            html`<option ?selected=${sample === this.selectedSample} value=${sample}>${sample}</option>`
          )}
        </select>
        <div class="pads-container">
          ${this.track.slots.map((vel, index) => {
            let idx = (index + 1) % this.track.slots.length;
            return html`
              <drum-pad 
                vel=${vel}
                ?trigger=${this.track.idx === idx}
                @pad-toggled=${(e: CustomEvent) => this._handlePadToggled(index, e.detail.velocity)}
              ></drum-pad>`
          })}
        </div>
      </div>
    `;
  }

  _handlePadToggled(index: number, velocity: number) {
    this.dispatchEvent(new CustomEvent('track-pad-toggled', {
      detail: {
        trackId: this.trkId,
        slotIdx: index,
        velocity: velocity
      },
      bubbles: true,
      composed: true
    }));
  }

  _handleSampleChange(event: Event) {
    const selectElement = event.target as HTMLSelectElement;
    const selectedValue = selectElement.value;
    this.dispatchEvent(new CustomEvent('sample-changed', {
      detail: {
        trackId: this.trkId,
        samplePath: selectedValue
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
