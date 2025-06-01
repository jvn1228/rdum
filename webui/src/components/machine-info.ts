import { LitElement, html, css } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { DrumMachineState } from '../models/types';

@customElement('machine-info')
export class MachineInfo extends LitElement {
  @property({ type: Object }) state?: DrumMachineState;
  
  // Properties for throttled latency updates
  private latencyUpdateInterval: number = 1000; // Update every 1000ms (1 second)
  private lastLatencyUpdateTime: number = 0;
  private displayedLatency: number = 0;

  static styles = css`
    :host {
      display: block;
      width: 100%;
    }
    
    .machine-info {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 10px;
      padding: 10px;
      color: var(--text-color);
    }
    
    .info-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 8px;
      background-color: var(--secondary-bg);
      border-radius: 4px;
    }
    
    .info-title {
      font-size: 0.8rem;
      opacity: 0.8;
      margin-bottom: 5px;
      text-transform: uppercase;
    }
    
    .info-value {
      font-size: 1rem;
      font-weight: bold;
    }
    
    .latency-low {
      color: var(--success-color, #4caf50);
    }
    
    .latency-medium {
      color: var(--warning-color, #ff9800);
    }
    
    .latency-high {
      color: var(--error-color, #f44336);
    }
  `;

  render() {
    if (!this.state) {
      return html`<div class="machine-info">Loading machine information...</div>`;
    }

    // Get current time
    const now = Date.now();
    
    // Check if it's time to update the latency display
    if (now - this.lastLatencyUpdateTime >= this.latencyUpdateInterval) {
      // Format latency for display (converting Duration to microseconds)
      const currentLatency = this.state.latency ? 
        typeof this.state.latency === 'number' ? 
          this.state.latency * 1000 : // Convert ms to μs if it's a number
          this.state.latency.secs !== undefined ? 
            (this.state.latency.secs * 1000000 + this.state.latency.nanos / 1000) : 
            0 : 
        0;
      
      // Update displayed latency and timestamp
      this.displayedLatency = currentLatency;
      this.lastLatencyUpdateTime = now;
    }
    
    // Determine latency class based on value (in microseconds)
    const latencyClass = this.displayedLatency < 5000 ? 
      'latency-low' : 
      this.displayedLatency < 15000 ? 
        'latency-medium' : 
        'latency-high';

    return html`
      <div class="machine-info">
        <div class="info-item">
          <div class="info-title">Pattern Length</div>
          <div class="info-value">${this.state.pattern_len} steps</div>
        </div>
        
        <div class="info-item">
          <div class="info-title">Division</div>
          <div class="info-value">${this.getDivisionName(this.state.division)}</div>
        </div>
        
        <div class="info-item">
          <div class="info-title">Track Count</div>
          <div class="info-value">${this.state.trks.length}</div>
        </div>
        
        <div class="info-item">
          <div class="info-title">Latency</div>
          <div class="info-value ${latencyClass}">${this.displayedLatency.toFixed(2)} μs</div>
        </div>
      </div>
    `;
  }

  // Convert division number to human-readable name
  getDivisionName(division: number): string {
    switch (division) {
      case 1: return 'Whole';
      case 2: return 'Half';
      case 3: return 'Quarter Dotted';
      case 4: return 'Quarter';
      case 6: return 'Eighth Dotted';
      case 8: return 'Eighth';
      case 12: return 'Sixteenth Dotted';
      case 16: return 'Sixteenth';
      case 24: return 'Thirty-Second Dotted';
      case 32: return 'Thirty-Second';
      default: return `${division}`;
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'machine-info': MachineInfo;
  }
}
