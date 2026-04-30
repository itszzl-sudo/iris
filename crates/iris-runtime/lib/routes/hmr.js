import { WebSocketServer } from 'ws';

export function handleFileChanges(changes, wsServer, cache, projectRoot) {
  if (!wsServer) return;
  for (const change of changes) {
    if (change.type === 'full-reload') {
      broadcast(wsServer, { type: 'full-reload', timestamp: Date.now() });
      break;
    }
    broadcast(wsServer, { type: change.type, path: change.filePath, timestamp: change.timestamp || Date.now() });
  }
}

function broadcast(wsServer, message) {
  const data = JSON.stringify(message);
  wsServer.clients.forEach((client) => {
    if (client.readyState === 1) client.send(data);
  });
}

export function setupWebSocketUpgrade(server, cache, projectRoot) {
  const wss = new WebSocketServer({ noServer: true });

  server.on('upgrade', (request, socket, head) => {
    if (request.url === '/@hmr') {
      wss.handleUpgrade(request, socket, head, (ws) => {
        wss.emit('connection', ws, request);
        console.log('  [HMR] Client connected');
        ws.on('message', (data) => {
          try {
            const msg = JSON.parse(data.toString());
            if (msg.type === 'ping') ws.send(JSON.stringify({ type: 'pong' }));
          } catch (_) {}
        });
        ws.on('close', () => console.log('  [HMR] Client disconnected'));
        ws.on('error', (err) => console.error('  [HMR] WebSocket error:', err.message));
        ws.send(JSON.stringify({ type: 'connected', timestamp: Date.now() }));
      });
    } else {
      socket.destroy();
    }
  });

  return wss;
}
