import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { Pattern } from '../models/types';
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';

@customElement('pattern-selector')
export class PatternSelector extends LitElement {
  @property({ type: Array }) patterns: Pattern[] = [];
  @property({ type: Number }) currentPatternId = 0;

  static styles = css`
    :host {
      display: block;
    }

    .pattern-selector {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin: 16px 0;
    }

    mwc-button {
      margin-right: 8px;
    }
  `;

  render() {
    return html`
      <div class="pattern-selector">
        ${this.patterns.map(pattern => {
          const isSelected = pattern.id === this.currentPatternId;
          return isSelected ? html`
            <md-filled-button
              @click=${() => this._handlePatternSelect(pattern.id)}
            >
              ${pattern.name}
            </md-filled-button>
          ` : html`
            <md-outlined-button
              @click=${() => this._handlePatternSelect(pattern.id)}
            >
              ${pattern.name}
            </md-outlined-button>
          `;
        })}
      </div>
    `;
  }

  _handlePatternSelect(patternId: number) {
    this.dispatchEvent(new CustomEvent('pattern-selected', {
      detail: { patternId },
      bubbles: true,
      composed: true
    }));
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'pattern-selector': PatternSelector;
  }
}
