#!/usr/bin/env node

import { startDevServer } from '../lib/server.js';
import chalk from 'chalk';

const args = process.argv.slice(2);

if (args[0] !== 'dev') {
  console.log(chalk.yellow('Usage: npx iris-runtime dev'));
  console.log();
  console.log(chalk.cyan('Options:'));
  console.log('  --port <number>    Server port (default: 3000, 0 for auto)');
  console.log('  --host <string>    Server host (default: localhost)');
  console.log('  --no-open          Do not open browser');
  console.log('  --no-hmr           Disable hot module replacement');
  console.log('  --debug            Enable debug output');
  console.log();
  process.exit(0);
}

const config = {
  port: 3000,
  host: 'localhost',
  open: true,
  enableHmr: true,
  debug: false,
  root: process.cwd(),
};

for (let i = 1; i < args.length; i++) {
  switch (args[i]) {
    case '--port':
      if (args[i + 1]) config.port = args[++i] === '0' ? 0 : parseInt(args[i]);
      break;
    case '--host':
      if (args[i + 1]) config.host = args[++i];
      break;
    case '--no-open':
      config.open = false;
      break;
    case '--no-hmr':
      config.enableHmr = false;
      break;
    case '--debug':
      config.debug = true;
      break;
  }
}

startDevServer(config).catch(error => {
  console.error(chalk.red('\n❌ Failed to start dev server:'));
  console.error(error.message);
  process.exit(1);
});
