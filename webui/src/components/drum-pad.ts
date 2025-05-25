import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';

@customElement('drum-pad')
export class DrumPad extends LitElement {
  @property({ type: Number, reflect: true }) vel = 0;
  @property({ type: Boolean, reflect: true }) trigger = false;

  static styles = css`
    :host {
      display: block;
    }

    .pad {
      width: 40px;
      height: 40px;
      border-radius: 4px;
      background-color: var(--drum-pad-inactive, #e0e0e0);
      cursor: pointer;
      transition: background-color 0.1s ease;
      border: none;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    }

    .pad.active {
      background-color: var(--drum-pad-active, #03dac6);
    }

    .pad.trigger {
      background-color: var(--drum-pad-trigger, #ff9800);
    }
  `;

  render() {
    const classes = {
      pad: true,
      active: this.vel > 0,
      trigger: this.trigger
    };
    
    return html`
      <button 
        class=${Object.entries(classes)
          .filter(([_, value]) => value)
          .map(([key]) => key)
          .join(' ')}
        @click=${this._handleClick}
      >${this.vel}</button>
    `;
  }

  _handleClick() {
    this.vel = this.vel === 0 ? 127 : 0;
    this.dispatchEvent(new CustomEvent('pad-toggled', {
      detail: { value: this.vel },
      bubbles: true,
      composed: true
    }));
  }

  triggerAnimation() {
    this.trigger = true;
    setTimeout(() => {
      this.trigger = false;
    }, 100);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'drum-pad': DrumPad;
  }
}
