import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import '@material/web/icon/icon.js';
import '@material/web/iconbutton/filled-icon-button.js';

/**
 * Theme toggle component for switching between light and dark modes
 */
@customElement('theme-switch')
export class ThemeSwitch extends LitElement {
  @property({ type: String }) theme: 'light' | 'dark' | 'system' = 'system';

  static styles = css`
    :host {
      display: block;
    }

    .theme-toggle {
      color: var(--text-primary-color);
      display: flex;
      align-items: center;
    }

    mwc-icon-button {
      color: inherit;
    }

    .tooltip {
      position: relative;
      display: inline-block;
    }

    .tooltip .tooltip-text {
      visibility: hidden;
      background-color: var(--tooltip-background);
      color: var(--tooltip-color);
      text-align: center;
      border-radius: 4px;
      padding: 5px 10px;
      position: absolute;
      z-index: 1;
      bottom: 125%;
      left: 50%;
      transform: translateX(-50%);
      opacity: 0;
      transition: opacity 0.2s;
      white-space: nowrap;
      font-size: 12px;
    }

    .tooltip:hover .tooltip-text {
      visibility: visible;
      opacity: 1;
    }
  `;

  constructor() {
    super();
    this._initializeTheme();
  }

  render() {
    return html`
      <div class="theme-toggle tooltip">
        <md-filled-icon-button class="icon-button" @click=${this._toggleTheme}>
          <span class="material-icons">${this._getThemeIcon()}</span>
        </md-filled-icon-button>
        <span class="tooltip-text">${this._getTooltipText()}</span>
      </div>
    `;
  }

  _initializeTheme() {
    // Check if there's a saved theme preference
    const savedTheme = localStorage.getItem('theme-preference');
    
    if (savedTheme === 'light' || savedTheme === 'dark' || savedTheme === 'system') {
      this.theme = savedTheme;
    } else {
      // Set default to system
      this.theme = 'system';
    }
    
    this._applyTheme();
  }

  _toggleTheme() {
    // Cycle through themes: light -> dark -> system -> light
    if (this.theme === 'light') {
      this.theme = 'dark';
    } else if (this.theme === 'dark') {
      this.theme = 'system';
    } else {
      this.theme = 'light';
    }
    
    // Save preference
    localStorage.setItem('theme-preference', this.theme);
    
    // Apply the new theme
    this._applyTheme();
    
    // Dispatch event for parent components
    this.dispatchEvent(new CustomEvent('theme-changed', {
      detail: { theme: this.theme },
      bubbles: true,
      composed: true
    }));
  }

  _applyTheme() {
    const root = document.documentElement;
    
    if (this.theme === 'system') {
      // Remove data-theme attribute to use system preference
      root.removeAttribute('data-theme');
    } else {
      // Set data-theme to force light or dark
      root.setAttribute('data-theme', this.theme);
    }
  }

  _getThemeIcon() {
    switch (this.theme) {
      case 'light':
        return 'light_mode';
      case 'dark':
        return 'dark_mode';
      case 'system':
        return 'settings_suggest';
      default:
        return 'settings_suggest';
    }
  }

  _getTooltipText() {
    switch (this.theme) {
      case 'light':
        return 'Light Mode (Click to switch)';
      case 'dark':
        return 'Dark Mode (Click to switch)';
      case 'system':
        return 'System Preference (Click to switch)';
      default:
        return 'Toggle Theme';
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'theme-switch': ThemeSwitch;
  }
}
