import { createServer } from 'net';

export function isPortInUse(port) {
  return new Promise((resolve) => {
    const server = createServer();
    server.once('error', (err) => { resolve(err.code === 'EADDRINUSE'); server.close(); });
    server.once('listening', () => { server.close(); resolve(false); });
    server.listen(port);
  });
}

export function findAvailablePort(startPort) {
  return new Promise((resolve, reject) => {
    const tryPort = (port) => {
      const server = createServer();
      server.once('error', (err) => {
        if (err.code === 'EADDRINUSE' && port < startPort + 100) tryPort(port + 1);
        else if (err.code === 'EADDRINUSE') reject(new Error('No available ports found'));
        else reject(err);
        server.close();
      });
      server.once('listening', () => { server.close(); resolve(port); });
      server.listen(port);
    };
    tryPort(startPort);
  });
}
