// Minimal backend: Redis keyspace notification to WebSocket push (TypeScript)
// Usage: npm start (with ts-node)
// Requirements: npm install

import WebSocket, { WebSocketServer } from 'ws';
import Redis from 'ioredis';

const redisHost = process.env.REDIS_HOST || '127.0.0.1';
const redis = new Redis({ host: redisHost });
const sub = new Redis({ host: redisHost }); // separate connection for pub/sub

const wss = new WebSocketServer({ port: 3002 });

// Enable keyspace notifications in Redis before running this:
// redis-cli CONFIG SET notify-keyspace-events K$

wss.on('connection', (ws: WebSocket) => {
  console.log('WebSocket client connected');
});

// Listen for set events on all keys (adjust pattern as needed)
sub.psubscribe('__keyspace@0__:*', (err, _count) => {
  if (err) console.error('Redis psubscribe error:', err);
});

sub.on('pmessage', async (_pattern: string, channel: string, message: string) => {
  // channel: __keyspace@0__:<key>
  // message: e.g. 'set', 'del', etc.
  if (message === 'set') {
    const key = channel.split(':').slice(1).join(':');
    try {
      const value = await redis.get(key);
      if (!value) return;
      // value is hex-encoded PNG, decode to base64 for browser
      const buf = Buffer.from(value, 'hex');
      const base64 = buf.toString('base64');
      // Broadcast to all clients
      wss.clients.forEach((client) => {
        if (client.readyState === WebSocket.OPEN) {
          client.send(JSON.stringify({ key, image: base64 }));
        }
      });
      console.log(`Pushed update for key ${key}`);
    } catch (e) {
      console.error('Error fetching key from Redis:', e);
    }
  }
});

console.log('WebSocket server running on ws://0.0.0.0:3002');
