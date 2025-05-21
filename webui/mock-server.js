// This is a simple WebSocket server for development
const WebSocket = require('ws');

const PORT = 8080;
const wss = new WebSocket.Server({ port: PORT });

// Store connected clients
const clients = new Set();

// Initial mock state
const mockState = {
  patterns: [
    {
      id: 1,
      name: 'Pattern 1',
      tracks: [
        { id: 1, name: 'Kick', slots: Array(16).fill(false).map((_, i) => i % 4 === 0) },
        { id: 2, name: 'Snare', slots: Array(16).fill(false).map((_, i) => i % 4 === 2) },
        { id: 3, name: 'Hi-Hat', slots: Array(16).fill(false).map((_, i) => i % 2 === 0) },
        { id: 4, name: 'Clap', slots: Array(16).fill(false) },
      ]
    },
    {
      id: 2,
      name: 'Pattern 2',
      tracks: [
        { id: 1, name: 'Kick', slots: Array(16).fill(false).map((_, i) => i % 8 === 0) },
        { id: 2, name: 'Snare', slots: Array(16).fill(false).map((_, i) => i % 8 === 4) },
        { id: 3, name: 'Hi-Hat', slots: Array(16).fill(false).map((_, i) => i % 4 === 0) },
        { id: 4, name: 'Clap', slots: Array(16).fill(false).map((_, i) => i % 4 === 3) },
      ]
    }
  ],
  currentPatternId: 1,
  isPlaying: false,
  currentStep: -1,
  tempo: 120
};

// Current state
let currentState = { ...mockState };
let sequencerInterval = null;

// Broadcast state to all clients
function broadcastState() {
  const message = {
    type: 'state_update',
    payload: currentState
  };
  
  const messageString = JSON.stringify(message);
  
  clients.forEach(client => {
    if (client.readyState === WebSocket.OPEN) {
      client.send(messageString);
    }
  });
}

// Start the sequencer
function startSequencer() {
  if (sequencerInterval) {
    clearInterval(sequencerInterval);
  }
  
  let step = 0;
  currentState.isPlaying = true;
  
  sequencerInterval = setInterval(() => {
    currentState.currentStep = step;
    broadcastState();
    step = (step + 1) % 16;
  }, (60 * 1000) / currentState.tempo / 4); // 16th notes
}

// Stop the sequencer
function stopSequencer() {
  if (sequencerInterval) {
    clearInterval(sequencerInterval);
    sequencerInterval = null;
  }
  
  currentState.isPlaying = false;
  currentState.currentStep = -1;
  broadcastState();
}

// Handle WebSocket connections
wss.on('connection', (ws) => {
  console.log('Client connected');
  clients.add(ws);
  
  // Send initial state to the client
  ws.send(JSON.stringify({
    type: 'state_update',
    payload: currentState
  }));
  
  // Handle messages from the client
  ws.on('message', (messageData) => {
    try {
      const message = JSON.parse(messageData.toString());
      console.log('Received message:', message);
      
      switch (message.type) {
        case 'toggle_pad':
          const { patternId, trackId, slotIndex, value } = message.payload;
          const pattern = currentState.patterns.find(p => p.id === patternId);
          if (pattern) {
            const track = pattern.tracks.find(t => t.id === trackId);
            if (track) {
              track.slots[slotIndex] = value;
              broadcastState();
            }
          }
          break;
          
        case 'change_pattern':
          currentState.currentPatternId = message.payload.patternId;
          broadcastState();
          break;
          
        case 'play':
          startSequencer();
          break;
          
        case 'stop':
          stopSequencer();
          break;
          
        case 'change_tempo':
          currentState.tempo = message.payload.tempo;
          
          // If we're playing, restart the sequencer with the new tempo
          if (currentState.isPlaying) {
            startSequencer();
          }
          
          broadcastState();
          break;
      }
    } catch (error) {
      console.error('Error parsing message:', error);
    }
  });
  
  // Handle client disconnection
  ws.on('close', () => {
    console.log('Client disconnected');
    clients.delete(ws);
  });
});

console.log(`WebSocket server running on ws://localhost:${PORT}`);
