import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/select/filled-select.js';
import '@material/web/select/select-option.js';

@customElement('pattern-selector')
export class PatternSelector extends LitElement {
  @property({ type: Number }) patternLen = 0;
  @property({ type: Number }) currentPatternId = 0;
  @property({ type: Number }) queuedPatternId = 0;
  @property({ type: String }) patternName = '';
  @property({ type: Array }) savedPatterns: string[] = [];

  private selectedPatternName: string = '';

  static styles = css`
    :host {
      display: block;
      padding: 16px;
      background-color: var(--surface-color);
      border-radius: 8px;
      box-shadow: 0 2px 4px var(--shadow-color);
    }
    .selector-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }
    .pattern-name {
      font-size: 1.2em;
      font-weight: 500;
    }
    .pattern-buttons {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-bottom: 1em;
    }
    @keyframes pulseEffect {
      0% {
        box-shadow: 0 0 0 0 rgba(var(--secondary-rgb, 3, 218, 198), 0.7);
        /* Using secondary-rgb from your main.css, or a fallback if not defined */
      }
      70% {
        box-shadow: 0 0 0 10px rgba(var(--secondary-rgb, 3, 218, 198), 0);
      }
      100% {
        box-shadow: 0 0 0 0 rgba(var(--secondary-rgb, 3, 218, 198), 0);
      }
    }

    md-filled-button.queued {
      --md-filled-button-container-color: var(--secondary-color);
      animation: pulseEffect 1s infinite;
      /* If text contrast is an issue with secondary-color, uncomment and set appropriately:
      --md-filled-button-label-text-color: var(--text-on-secondary-color); 
      */
    }
  `;

  render() {
    return html`
      <div class="pattern-selector">
        <div class="pattern-buttons">
          ${Array.from({ length: this.patternLen }, (_, i) => {
            const isSelected = i === this.currentPatternId;
            const isQueuedAndNotSelected = i === this.queuedPatternId && i !== this.currentPatternId;

            if (isSelected) {
              return html`
                <md-filled-button class="pattern-button selected" @click=${() => this._handlePatternSelect(i)}>
                  Pattern ${i + 1}
                </md-filled-button>`;
            } else if (isQueuedAndNotSelected) {
              return html`
                <md-filled-button class="pattern-button queued" @click=${() => this._handlePatternSelect(i)}>
                  Pattern ${i + 1}
                </md-filled-button>`;
            } else {
              return html`
                <md-outlined-button class="pattern-button" @click=${() => this._handlePatternSelect(i)}>
                  Pattern ${i + 1}
                </md-outlined-button>`;
            }
          })}
        </div>
        <md-filled-button @click=${() => this._handleAddPattern()}>
          <md-icon slot="icon">add</md-icon>
          Add Pattern
        </md-filled-button>
        <md-filled-button @click=${() => this._handleSavePattern()}>
          <md-icon slot="icon">save</md-icon>
          Save Pattern
        </md-filled-button>
        <md-filled-select @change=${this._handleSavedPatternSelect}>
          ${this.savedPatterns.map((pattern) => html`
            <md-select-option .value=${pattern} .label=${pattern}>${pattern}</md-select-option>
          `)}
        </md-filled-select>
        <md-filled-button @click=${() => this._handleLoadPattern()}>
          <md-icon slot="icon">download</md-icon>
          Load Pattern
        </md-filled-button>
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

  _handleSavePattern() {
    this.dispatchEvent(new CustomEvent('save-pattern', {
      bubbles: true,
      composed: true
    }));
  }

  _handleSavedPatternSelect(e: Event) {
    const select = e.target as HTMLSelectElement;
    const selectedOption = select.options[select.selectedIndex];
    this.selectedPatternName = selectedOption.value;
  }

  _handleLoadPattern() {
    this.dispatchEvent(new CustomEvent('load-pattern', {
      detail: { fname: this.selectedPatternName },
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
