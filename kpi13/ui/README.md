
# Fractal Tile Viewer: Redis + WebSocket Architecture

A React application that displays 9 image tiles in a 3x3 grid, each updated in real time from Redis via a WebSocket backend. The app features a real-time log system and follows Siemens design guidelines.


## Architecture Overview

This system visualizes a zooming Mandelbrot fractal in a 3x3 tile grid, using Redis as the image buffer, a WebSocket backend for real-time updates, and a React frontend for display. The architecture is as follows:

1. **Fractal Tile Generator (tester-mandelbrot-redis)**
   - Continuously generates 9 Mandelbrot PNG images (tiles) zooming into interesting fractal locations.
   - Pushes each tile as a PNG (hex-encoded) to Redis keys `1` through `9` every 5 seconds.

2. **Redis WebSocket Backend**
   - Listens for changes to Redis keys (using keyspace notifications).
   - When a tile image is updated, pushes the new image to all connected frontend clients via WebSocket.

3. **Frontend Tile Viewer (React App)**
   - Connects to the WebSocket backend.
   - Receives tile image updates and displays them in a 3x3 grid.
   - Includes real-time logs and follows Siemens design guidelines.

### Data Flow Diagram

```
┌────────────────────────────┐      ┌────────────────────────────┐      ┌────────────────────────────┐
│ Fractal Tile Generator    │      │ Redis                      │      │ WebSocket Backend         │
│ (tester-mandelbrot-redis) │───▶──│ 1,2,...,9 (PNG hex)        │───▶──│ Pushes tile updates       │
└────────────────────────────┘      └────────────────────────────┘      └────────────────────────────┘
                                                                              │
                                                                              ▼
                                                                ┌────────────────────────────┐
                                                                │ Frontend Tile Viewer (UI)  │
                                                                └────────────────────────────┘
```

## Features

- **3x3 Tile Grid**: Square image tiles arranged in a responsive grid
- **Fractal Animation**: Tiles update every 5 seconds, zooming into infinite Mandelbrot details
- **Redis/WebSocket Integration**: Real-time updates via Redis and WebSocket backend
- **Real-time Logs**: Continuous logging system showing connection status and tile updates
- **Siemens Design**: Professional color scheme following Siemens brand guidelines
- **Responsive Layout**: Adapts to different screen sizes

## Layout

```
┌─────────────────────────────────────────────────────────────┐
│                    Fractal Tile Viewer                      │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ │ ┌──────────────┐     │
│  │ Tile 1  │ │ Tile 2  │ │ Tile 3  │ │ │              │     │
│  │         │ │         │ │         │ │ │              │     │
│  └─────────┘ └─────────┘ └─────────┘ │ │   System     │     │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ │ │    Logs      │     │
│  │ Tile 4  │ │ Tile 5  │ │ Tile 6  │ │ │              │     │
│  │         │ │         │ │         │ │ │              │     │
│  └─────────┘ └─────────┘ └─────────┘ │ │              │     │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ │ │              │     │
│  │ Tile 7  │ │ Tile 8  │ │ Tile 9  │ │ │              │     │
│  │         │ │         │ │         │ │ │              │     │
│  └─────────┘ └─────────┘ └─────────┘ │ └──────────────┘     │
├─────────────────────────────────────────────────────────────┤
```


## Redis Keys and WebSocket Messages

Each tile is stored in Redis as a PNG hex string under keys `1` through `9`.

When a tile is updated, the backend pushes a WebSocket message to the frontend:

```
{
   "tile": 1, // 1-9
   "imageHex": "<hex-encoded PNG>"
}
```

## Installation


### Option 1: Docker Compose (Recommended)

The included `docker-compose.yml` launches:
- The fractal tile generator (tester)
- Redis
- The WebSocket backend
- The React frontend

To start all services:

```
docker compose up --build
```

**Tunneling for Local Access:**
If running remotely, tunnel both the web UI and WebSocket backend:

```
ssh -L 3000:localhost:3000 -L 3002:localhost:3002 user@server
```

Then access the web UI at `http://localhost:3000` and the WebSocket at `ws://localhost:3002` from your browser.



1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd fractal-tile-viewer
   ```

2. **Start all services**:
   ```bash
   ./start.sh
   ```

3. **Access the application**:
   - WebUI: http://localhost:3000
   - Redis WebSocket Backend: ws://localhost:3002

4. **Stop all services**:
   ```bash
   ./stop.sh
   ```

### Option 2: Local Development

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd fractal-tile-viewer
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Start the development server**:
   ```bash
   npm start
   ```

4. **Build for production**:
   ```bash
   npm run build
   ```

## Usage

1. **Connect to Redis WebSocket Backend**:
   - Enter your Redis WebSocket backend URL (e.g., `ws://localhost:3002`)
   - Click "Connect" to establish connection

2. **Update Tiles**:
   - When new image chunks are written to Redis, the backend will push updates to the UI automatically

3. **Monitor Logs**:
   - View real-time system logs in the right column
   - Logs show connection status and tile updates


## Fractal Test Generator

The included `tester-mandelbrot-redis` service continuously generates Mandelbrot tile images and pushes them to Redis, demonstrating the infinite fractal structure. This provides a visually rich, real-time test for the entire stack.

### Features
- **Animated Fractal Zoom**: Tiles update every 5 seconds, zooming into interesting Mandelbrot locations
- **Redis/WebSocket Integration**: Real-time updates to the frontend
- **Easy Verification**: The fractal structure makes it easy to see that updates are working

### How It Works
1. The generator writes 9 PNG tiles (as hex) to Redis keys 1-9
2. The backend pushes updates to the frontend via WebSocket
3. The frontend displays the updated tiles in a 3x3 grid



## Configuration



### Siemens Color Scheme

The app uses Siemens brand colors defined in CSS variables:

- Primary Blue: `#009999`
- Dark Blue: `#006666`
- Light Blue: `#66cccc`
- Status Colors: Green (connected), Orange (disconnected), Red (error)

## Dependencies

- React 18.2.0
- TypeScript 4.9.0

- React Scripts 5.0.1

## Browser Support

- Chrome (recommended)
- Firefox
- Safari
- Edge

## Troubleshooting

### Connection Issues



### Image Loading Issues

1. **CORS**: Ensure image URLs allow cross-origin requests
2. **Image Format**: Use common formats (JPEG, PNG, GIF)
3. **Fallback**: The app includes placeholder images for testing

### Performance

1. **Image Size**: Keep images under 1MB for optimal performance
2. **Update Frequency**: Avoid extremely frequent updates (recommended: < 1Hz per tile)
3. **Browser Memory**: Monitor memory usage with many high-resolution images

## Development

### Project Structure

```
src/
├── App.tsx          # Main application component
├── App.css          # Styles with Siemens design system
├── index.tsx        # Application entry point
└── index.css        # Global styles
```

### Adding Features

1. **New Tile Properties**: Extend the `TileData` interface
2. **Custom MQTT Messages**: Modify the message parsing logic
3. **Additional Logging**: Extend the `LogEntry` interface and logging functions

## License

This project is licensed under the MIT License.

## Support

For issues and questions, please check the troubleshooting section or create an issue in the repository.
