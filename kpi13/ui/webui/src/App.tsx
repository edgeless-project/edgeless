import React, { useState, useEffect, useRef } from 'react';
// import mqtt, { MqttClient } from 'mqtt';
import './App.css';
import { IxApplication, IxApplicationHeader, IxButton, IxContent, IxContentHeader, IxIcon, IxInput, IxTextarea } from '@siemens/ix-react';
import { iconConnected, iconConnectionFail } from '@siemens/ix-icons/icons';

interface TileData {
  id: number;
  topic: string;
  imageUrl: string;
  lastUpdate: Date;
  previousUpdate?: Date;
  status: 'connected' | 'disconnected' | 'error';
  timeSinceUpdate: string;
}

interface TileStats {
  tileId: number;
  interarrivalTimes: number[]; // in milliseconds
  avgInterarrival: number;
  minInterarrival: number;
  maxInterarrival: number;
  totalUpdates: number;
}

interface LogEntry {
  id: string;
  timestamp: Date;
  message: string;
  level: 'info' | 'warning' | 'error';
}

const App: React.FC = () => {
  const [tiles, setTiles] = useState<TileData[]>([]);
  const [tileStats, setTileStats] = useState<TileStats[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  // Redis WebSocket connection
  const [ws, setWs] = useState<WebSocket | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'connected' | 'disconnected'>('disconnected');
  const [wsUrl, setWsUrl] = useState('ws://localhost:3002');
  const logContainerRef = useRef<HTMLDivElement>(null);

  // Function to calculate time since last update
  const calculateTimeSinceUpdate = (lastUpdate: Date): string => {
    const now = new Date();
    const diffInMs = now.getTime() - lastUpdate.getTime();
    
    if (diffInMs < 1000) {
      return `${diffInMs}ms ago`;
    } else if (diffInMs < 60000) {
      const seconds = Math.floor(diffInMs / 1000);
      const remainingMs = diffInMs % 1000;
      return `${seconds}.${remainingMs.toString().padStart(3, '0')}s ago`;
    } else if (diffInMs < 3600000) {
      const minutes = Math.floor(diffInMs / 60000);
      const remainingSeconds = Math.floor((diffInMs % 60000) / 1000);
      return `${minutes}m ${remainingSeconds}s ago`;
    } else {
      const hours = Math.floor(diffInMs / 3600000);
      const remainingMinutes = Math.floor((diffInMs % 3600000) / 60000);
      return `${hours}h ${remainingMinutes}m ago`;
    }
  };

  // Initialize tiles with default data
  useEffect(() => {
    const initialTiles: TileData[] = Array.from({ length: 9 }, (_, index) => ({
      id: index,
      topic: `tile/${index + 1}`,
      imageUrl: '', // Start with no image
      lastUpdate: new Date(),
      status: 'disconnected',
      timeSinceUpdate: '0s ago'
    }));
    setTiles(initialTiles);

    // Initialize tile statistics
    const initialStats: TileStats[] = Array.from({ length: 9 }, (_, index) => ({
      tileId: index,
      interarrivalTimes: [],
      avgInterarrival: 0,
      minInterarrival: 0,
      maxInterarrival: 0,
      totalUpdates: 0
    }));
    setTileStats(initialStats);
  }, []);

  // Redis WebSocket connection setup
  const connectToWs = () => {
    if (ws) {
      ws.close();
    }
    setConnectionStatus('connecting');
    try {
      const socket = new window.WebSocket(wsUrl);
      socket.onopen = () => {
        setConnectionStatus('connected');
        addLog('Connected to Redis WebSocket backend', 'info');
        // Mark all tiles as connected
        tiles.forEach(tile => updateTileStatus(tile.id, 'connected'));
      };

      socket.onmessage = (event) => {
        console.log("hello");
        try {
          const data = JSON.parse(event.data);
          // Expecting { key, image }
          // key: e.g. "1" or "tile/1" or similar
          let tileIndex = -1;
          if (typeof data.key === 'string') {
            // Try to extract tile index from key
            const match = data.key.match(/(\d+)/);
            if (match) {
              tileIndex = parseInt(match[1], 10) - 1;
            }
          }
          console.log("got data for tileIndex", tileIndex);
          if (tileIndex >= 0 && tileIndex < tiles.length && data.image) {
            updateTileImage(tileIndex, `data:image/png;base64,${data.image}`);
            addLog(`Updated tile ${tileIndex + 1} with fractal data (Redis, ${data.image.length} chars)`, 'info');
          } else {
            addLog(`Received Redis WS message for unknown tile key: ${data.key}`, 'warning');
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          addLog(`Invalid Redis WS message: ${errorMessage}`, 'error');
        }
      };

      socket.onerror = (error) => {
        setConnectionStatus('disconnected');
        addLog(`Redis WebSocket error: ${JSON.stringify(error)}`, 'error');
      };
      socket.onclose = () => {
        setConnectionStatus('disconnected');
        addLog('Redis WebSocket connection closed', 'warning');
        tiles.forEach(tile => updateTileStatus(tile.id, 'disconnected'));
      };
      setWs(socket);
    } catch (error) {
      setConnectionStatus('disconnected');
      const errorMessage = error instanceof Error ? error.message : String(error);
      addLog(`Failed to connect to Redis WebSocket: ${errorMessage}`, 'error');
    }
  };

  const disconnectFromWs = () => {
    if (ws) {
      ws.close();
      setWs(null);
      setConnectionStatus('disconnected');
      addLog('Disconnected from Redis WebSocket backend', 'info');
      tiles.forEach(tile => updateTileStatus(tile.id, 'disconnected'));
    }
  };

  const updateTileImage = (tileId: number, imageUrl: string) => {
    const now = new Date();
    
    // Update tile data and track interarrival time
    setTiles(prev => prev.map(tile => {
      if (tile.id === tileId) {
        // Calculate interarrival time if this isn't the first update
        let interarrivalTime = 0;
        if (tile.previousUpdate) {
          interarrivalTime = now.getTime() - tile.previousUpdate.getTime();
          
          // Update statistics
          setTileStats(prevStats => prevStats.map(stat => {
            if (stat.tileId === tileId) {
              const newTimes = [...stat.interarrivalTimes, interarrivalTime];
              // Keep only last 50 interarrival times for histogram
              const trimmedTimes = newTimes.slice(-50);
              
              return {
                ...stat,
                interarrivalTimes: trimmedTimes,
                avgInterarrival: trimmedTimes.reduce((sum, time) => sum + time, 0) / trimmedTimes.length,
                minInterarrival: Math.min(...trimmedTimes),
                maxInterarrival: Math.max(...trimmedTimes),
                totalUpdates: stat.totalUpdates + 1
              };
            }
            return stat;
          }));
        }
        
        return { 
          ...tile, 
          imageUrl, 
          previousUpdate: tile.lastUpdate,
          lastUpdate: now,
          timeSinceUpdate: calculateTimeSinceUpdate(now)
        };
      }
      return tile;
    }));
  };

  const updateTileStatus = (tileId: number, status: 'connected' | 'disconnected' | 'error') => {
    setTiles(prev => prev.map(tile => 
      tile.id === tileId 
        ? { ...tile, status }
        : tile
    ));
  };

  const addLog = (message: string, level: 'info' | 'warning' | 'error') => {
    const newLog: LogEntry = {
      id: Date.now().toString(),
      timestamp: new Date(),
      message,
      level
    };
    
    setLogs(prev => {
      const updated = [newLog, ...prev.slice(0, 19)]; // Keep last 20 logs, newest on top
      return updated;
    });
  };

  const formatTimestampWithMilliseconds = (date: Date): string => {
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    const milliseconds = date.getMilliseconds().toString().padStart(3, '0');
    return `${hours}:${minutes}:${seconds}.${milliseconds}`;
  };

  // Keep scroll position at top to show newest logs first
  useEffect(() => {
    if (logContainerRef.current) {
      logContainerRef.current.scrollTop = 0;
    }
  }, [logs]);

  // Update time since last update every 100ms for better millisecond precision
  useEffect(() => {
    const interval = setInterval(() => {
      setTiles(prev => prev.map(tile => ({
        ...tile,
        timeSinceUpdate: calculateTimeSinceUpdate(tile.lastUpdate)
      })));
    }, 100);

    return () => clearInterval(interval);
  }, []);

  return (
    <IxApplication theme="dark">
      <IxApplicationHeader name="Edgeless KPI#13 - Fractal Resilience Viewer">
        <div className="placeholder-logo" slot="logo"></div>
      </IxApplicationHeader>
      <IxContent>
             
                <IxContentHeader>
                <IxInput
                  type="text"
                  value={wsUrl}
                  onChange={(e) => setWsUrl((e.target as HTMLInputElement).value)}
                  placeholder="Redis WS Backend URL"
                />
                {connectionStatus === 'disconnected' ? (
                  <IxButton outline onClick={connectToWs}>
                    Connect
                  </IxButton>
                ) : (
                  <IxButton outline onClick={disconnectFromWs}>
                    Disconnect
                  </IxButton>
                )}
                <span className={`status-indicator ${connectionStatus}`}>
                  {connectionStatus}
                </span>
                {ws && (
                  <span>
                    Redis WS: {tiles.filter(t => t.status === 'connected').length}/9 tiles
                  </span>
                )}
              </IxContentHeader>

          <main className="app-main">
            <div className="top-section">
              <div className="tiles-container">
                <div className="tiles-grid">
                  {tiles.map((tile) => (
                    <div key={tile.id} className={`tile ${tile.status}`}>
                      <div className="tile-header">
                        <span className="tile-topic">{tile.topic}</span>
                        <IxIcon
                          name={tile.status === 'connected' ? 'iconConnected' : 'iconConnectionFail'}
                          color={tile.status === 'connected' ? 'green' : 'red'}
                        />
                      </div>
                      {tile.imageUrl && (
                        <div className="tile-data-info">
                          <span>Fractal Data: {tile.imageUrl.length} chars</span>
                        </div>
                      )}
                      {tile.imageUrl ? (
                        <img 
                          src={tile.imageUrl} 
                          alt={`Tile ${tile.id + 1}`}
                          className="tile-image"
                          onError={(e) => {
                            const target = e.target as HTMLImageElement;
                            target.classList.add('error');
                            target.style.display = 'flex';
                            target.style.alignItems = 'center';
                            target.style.justifyContent = 'center';
                            target.style.color = 'white';
                            target.style.fontSize = '0.8rem';
                            target.style.textAlign = 'center';
                            addLog(`Image failed to load for tile ${tile.id + 1}`, 'warning');
                          }}
                        />
                      ) : (
                        <div className="tile-loading">
                          <div className="loading-spinner"></div>
                          <span>Waiting for fractal data...</span>
                        </div>
                      )}
                      <div className="tile-footer">
                        <span className="tile-id">Tile {tile.id + 1}</span>
                        <span className="tile-time">
                          {tile.timeSinceUpdate}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              <div className="stats-container">
              <h3>Interarrival Time Statistics</h3>
              <div className="stats-grid">
                {tileStats.map((stat) => (
                  <div key={stat.tileId} className="tile-stat">
                    <div className="stat-header">
                      <span className="stat-title">Tile {stat.tileId + 1}</span>
                      <span className="stat-updates">({stat.totalUpdates} updates)</span>
                    </div>
                    {stat.interarrivalTimes.length > 0 ? (
                      <>
                        <div className="stat-values">
                          <div className="stat-item">
                            <span className="stat-label">Avg:</span>
                            <span className="stat-value">{stat.avgInterarrival.toFixed(0)}ms</span>
                          </div>
                          <div className="stat-item">
                            <span className="stat-label">Min:</span>
                            <span className="stat-value">{stat.minInterarrival}ms</span>
                          </div>
                          <div className="stat-item">
                            <span className="stat-label">Max:</span>
                            <span className="stat-value">{stat.maxInterarrival}ms</span>
                          </div>
                        </div>
                        <div className="histogram">
                          {(() => {
                            // Create histogram bins
                            const binCount = 10;
                            const range = stat.maxInterarrival - stat.minInterarrival;
                            const binSize = Math.max(1, Math.ceil(range / binCount));
                            const bins = new Array(binCount).fill(0);
                            
                            // Fill bins
                            stat.interarrivalTimes.forEach(time => {
                              const binIndex = Math.min(
                                binCount - 1,
                                Math.floor((time - stat.minInterarrival) / binSize)
                              );
                              bins[binIndex]++;
                            });
                            
                            const maxCount = Math.max(...bins);
                            
                            return bins.map((count, index) => (
                              <div 
                                key={index} 
                                className="histogram-bar"
                                style={{
                                  height: `${maxCount > 0 ? (count / maxCount) * 40 : 0}px`,
                                  backgroundColor: count > 0 ? '#00A3A0' : '#333'
                                }}
                                title={`${stat.minInterarrival + index * binSize}-${stat.minInterarrival + (index + 1) * binSize}ms: ${count} samples`}
                              />
                            ));
                          })()}
                        </div>
                      </>
                    ) : (
                      <div className="no-data">No data yet</div>
                    )}
                  </div>
                ))}
              </div>
            </div>
            </div>
            
            <div className="logs-container">
              <div className="logs-content" ref={logContainerRef}>
                {logs.map((log) => (
                  <div key={log.id}>
                    {formatTimestampWithMilliseconds(log.timestamp)} - {log.message}
                  </div>
                ))}
              </div>
            </div>
          </main>
      </IxContent>
    </IxApplication>
  );
};

export default App;
