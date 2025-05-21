# RDUM - Drum Machine Web UI

A TypeScript-based drum machine user interface using Lit components and Material Design. This UI communicates with a backend server via WebSockets to synchronize state and send user interactions.

## Features

- Pattern-based drum sequencer interface
- Interactive drum pads for creating beats
- Transport controls (play, stop, tempo adjustment)
- Multiple pattern support with easy navigation
- Real-time synchronization via WebSockets

## Tech Stack

- TypeScript
- Lit (Web Components)
- Material Design (Material Web Components)
- WebSockets for real-time communication
- Vite for development and building

## Project Structure

```
rdum/webui/
├── src/
│   ├── components/     # Lit components
│   ├── models/         # TypeScript interfaces and types
│   ├── services/       # WebSocket and other services
│   ├── styles/         # CSS styles
│   └── main.ts         # Main entry point
├── public/             # Static assets
├── index.html          # Main HTML file
├── package.json        # Dependencies and scripts
├── tsconfig.json       # TypeScript configuration
└── vite.config.ts      # Vite configuration
```

## Development

### Prerequisites

- Node.js (v14 or later)
- npm (v6 or later)

### Setup

1. Install dependencies:

```bash
npm install
```

2. Start the development server:

```bash
npm run dev
```

This will start the development server at http://localhost:3000.

### Building for Production

```bash
npm run build
```

The built files will be in the `dist` directory.

## WebSocket Communication

The UI communicates with the backend server using WebSockets. The protocol uses JSON messages with the following format:

```typescript
{
  type: MessageType,
  payload: any
}
```

The message types are defined in `src/models/types.ts` and include:

- `STATE_UPDATE`: Server sends updated state to clients
- `TOGGLE_PAD`: Client toggles a drum pad
- `CHANGE_PATTERN`: Client changes the current pattern
- `PLAY`: Client requests to start playback
- `STOP`: Client requests to stop playback
- `CHANGE_TEMPO`: Client changes the tempo

## Development Notes

For development without a backend, a mock WebSocket service is provided in `main.ts`. This allows for testing the UI functionality without needing to run the actual backend server.

To use with a real backend:
1. Comment out the `simulateMockData()` call in `main.ts`
2. Update the WebSocket URL in the WebSocketService constructor if needed

## License

ISC
