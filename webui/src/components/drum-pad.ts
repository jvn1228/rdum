import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';

@customElement('drum-pad')
export class DrumPad extends LitElement {
  @property({ type: Number, reflect: true }) vel = 0;
  @property({ type: Boolean, reflect: true }) trigger = false;

  private initialVelOnAdjust = 0;
  private pointerStartY = 0;

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
        @pointerdown=${this._handlePointerDown}
      >${this.vel}</button>
    `;
  }

  // Bound methods for window event listeners
  private _boundHandlePointerMove = this._handlePointerMove.bind(this);
  private _boundHandlePointerUp = this._handlePointerUp.bind(this);

  _handlePointerDown(e: PointerEvent) {
    // Prevent text selection/default drag behaviors if the button itself is clicked
    e.preventDefault(); 

    this.initialVelOnAdjust = this.vel;
    this.pointerStartY = e.clientY;

    // Capture pointer events on the window to handle dragging outside the element
    window.addEventListener('pointermove', this._boundHandlePointerMove);
    window.addEventListener('pointerup', this._boundHandlePointerUp);
    window.addEventListener('pointercancel', this._boundHandlePointerUp); // Also handle cancel
  }

  _handlePointerMove(e: PointerEvent) {
    const dy = Math.round(e.clientY - this.pointerStartY); // Positive dy if mouse moves down
    // Velocity increases as mouse drags UP, decreases as mouse drags DOWN.
    this.vel = Math.max(0, Math.min(127, this.initialVelOnAdjust - dy));
  }

  _handlePointerUp(e: PointerEvent) {
    if (Math.abs(e.clientY - this.pointerStartY) < 2) {
      this.vel = this.vel == 0 ? 127 : 0;
    }
    this.dispatchEvent(new CustomEvent('pad-toggled', {
      detail: { velocity: this.vel },
      bubbles: true,
      composed: true
    }));

    window.removeEventListener('pointermove', this._boundHandlePointerMove);
    window.removeEventListener('pointerup', this._boundHandlePointerUp);
    window.removeEventListener('pointercancel', this._boundHandlePointerUp);
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
