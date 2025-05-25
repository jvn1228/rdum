// Import components
import './components/drum-machine-app';
import './components/drum-pad';
import './components/drum-track';
import './components/pattern-selector';
import './components/transport-controls';
import './components/theme-switch';

// Import Material Web components 
import '@material/web/iconbutton/filled-icon-button.js';
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/slider/slider.js';
import 'material-symbols';
// For the WebComponents polyfill
if (!window.customElements) {
  document.write('<script src="https://unpkg.com/@webcomponents/webcomponentsjs@2.0.0/webcomponents-bundle.js"></script>');
}

// // Mock data for development - will be replaced by actual WebSocket data
// import { WebSocketService } from './services/websocket-service';
// import { DrumMachineState } from './models/types';

// // Create a mock function to simulate receiving WebSocket messages (for development)
// function simulateMockData() {
//   const mockState: DrumMachineState = {
//     patterns: [
//       {
//         id: 1,
//         name: 'Pattern 1',
//         tracks: [
//           { id: 1, name: 'Kick', slots: Array(16).fill(false).map((_, i) => i % 4 === 0) },
//           { id: 2, name: 'Snare', slots: Array(16).fill(false).map((_, i) => i % 4 === 2) },
//           { id: 3, name: 'Hi-Hat', slots: Array(16).fill(false).map((_, i) => i % 2 === 0) },
//           { id: 4, name: 'Clap', slots: Array(16).fill(false) },
//         ]
//       },
//       {
//         id: 2,
//         name: 'Pattern 2',
//         tracks: [
//           { id: 1, name: 'Kick', slots: Array(16).fill(false).map((_, i) => i % 8 === 0) },
//           { id: 2, name: 'Snare', slots: Array(16).fill(false).map((_, i) => i % 8 === 4) },
//           { id: 3, name: 'Hi-Hat', slots: Array(16).fill(false).map((_, i) => i % 4 === 0) },
//           { id: 4, name: 'Clap', slots: Array(16).fill(false).map((_, i) => i % 4 === 3) },
//         ]
//       }
//     ],
//     currentPatternId: 1,
//     isPlaying: false,
//     currentStep: -1,
//     tempo: 120
//   };

//   // Update WebSocketService prototype for development mocking
  
//   // For development only - extends the prototype for mock implementation
//   // We need to use 'any' type to access and extend private members for testing purposes
//   (WebSocketService.prototype as any)._mockNotifyListeners = function(state: DrumMachineState) {
//     // Using 'any' to access the private listeners array
//     (this.listeners as any[]).forEach((listener: (state: DrumMachineState) => void) => listener(state));
//   };
  
//   // Comment this out when connecting to a real WebSocket server
//   WebSocketService.prototype.connect = function() {
//     console.log('Using mock WebSocket data for development');
    
//     // Simulate initial state
//     setTimeout(() => {
//       (this as any)._mockNotifyListeners(mockState);
//     }, 500);
    
//     // Implement mock methods to update the UI locally for development
//     this.togglePad = function(patternId, trackId, slotIndex, value) {
//       const pattern = mockState.patterns.find(p => p.id === patternId);
//       if (pattern) {
//         const track = pattern.tracks.find(t => t.id === trackId);
//         if (track) {
//           track.slots[slotIndex] = value;
//           (this as any)._mockNotifyListeners({...mockState});
//         }
//       }
//     };
    
//     this.changePattern = function(patternId) {
//       mockState.currentPatternId = patternId;
//       (this as any)._mockNotifyListeners({...mockState});
//     };
    
//     this.play = function() {
//       mockState.isPlaying = true;
//       (this as any)._mockNotifyListeners({...mockState});
      
//       // Simulate sequencer
//       let step = 0;
//       const interval = setInterval(() => {
//         if (!mockState.isPlaying) {
//           clearInterval(interval);
//           return;
//         }
        
//         mockState.currentStep = step;
//         (this as any)._mockNotifyListeners({...mockState});
        
//         step = (step + 1) % 16;
//       }, (60 * 1000) / mockState.tempo / 4); // 16th notes
//     };
    
//     this.stop = function() {
//       mockState.isPlaying = false;
//       mockState.currentStep = -1;
//       (this as any)._mockNotifyListeners({...mockState});
//     };
    
//     this.changeTempo = function(tempo) {
//       mockState.tempo = tempo;
//       (this as any)._mockNotifyListeners({...mockState});
//     };
//   };
// }

// // Call this function during development when no backend is available
// // Comment it out when connecting to a real WebSocket server
// simulateMockData();

console.log('RDUM - Drum Machine UI initialized');
