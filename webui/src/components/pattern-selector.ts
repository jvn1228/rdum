import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';

@customElement('pattern-selector')
export class PatternSelector extends LitElement {
  @property({ type: Number }) patternLen = 0;
  @property({ type: Number }) currentPatternId = 0;
  @property({ type: String }) patternName = '';

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

    .pattern-buttons {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-right: 16px;
    }

    .pattern-info {
      margin-left: auto;
      display: flex;
      align-items: center;
    }
  `;

  render() {
    return html`
      <div class="pattern-selector">
        <div class="pattern-buttons">
          ${Array.from({ length: this.patternLen }, (_, i) => {
            const isSelected = i === this.currentPatternId;
            return isSelected ? html`
              <md-filled-button @click=${() => this._handlePatternSelect(i)}>
                Pattern ${i + 1}
              </md-filled-button>
            ` : html`
              <md-outlined-button @click=${() => this._handlePatternSelect(i)}>
                Pattern ${i + 1}
              </md-outlined-button>
            `;
          })}
        </div>
        <md-outlined-button @click=${() => this._handleAddPattern()}>Add Pattern</md-outlined-button>
      </div>
    `;
  }

  _handleAddPattern() {
    this.dispatchEvent(new CustomEvent('add-pattern', {
      bubbles: true,
      composed: true
    }));
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
