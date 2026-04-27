#!/usr/bin/env node

const { spawn, spawnSync } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');
const http = require('http');

// Detect platform
const platform = os.platform(); // 'win32', 'darwin', 'linux'
const arch = os.arch(); // 'x64', 'arm64'

const platformMap = {
  'win32': 'win-x64',
  'linux': 'linux-x64',
  'darwin': arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64'
};

const binaryPlatform = platformMap[platform];
if (!binaryPlatform) {
  console.error(`❌ Unsupported platform: ${platform} ${arch}`);
  console.error('Supported: Windows x64, Linux x64, macOS x64/arm64');
  process.exit(1);
}

const binaryDir = path.join(__dirname, '..', 'binaries', binaryPlatform);
const skyBinary = platform === 'win32' ? 'sky.exe' : 'sky';
const skyPath = path.join(binaryDir, skyBinary);
const skybridgeBinary = platform === 'win32' ? 'skybridge.exe' : 'skybridge';
const skybridgePath = path.join(binaryDir, skybridgeBinary);

// Check if binary exists
if (!fs.existsSync(skyPath)) {
  console.error(`❌ Sky-Code binary not found for ${binaryPlatform}`);
  console.error(`Expected: ${skyPath}`);
  console.error('\nTroubleshooting:');
  console.error('  1. Reinstall: npm install -g sky-code');
  console.error('  2. Check GitHub issues: https://github.com/sky-code/sky-code/issues');
  process.exit(1);
}

if (!fs.existsSync(skybridgePath)) {
  console.error(`❌ SkyBridge binary not found for ${binaryPlatform}`);
  console.error(`Expected: ${skybridgePath}`);
  process.exit(1);
}

// Setup environment variables
const env = {
  ...process.env,
  FILANTHROPIC_BASE_URL: process.env.FILANTHROPIC_BASE_URL || 'http://localhost:4000',
  FILANTHROPIC_API_KEY: process.env.FILANTHROPIC_API_KEY || 'ollama',
  FILANTHROPIC_MODEL: process.env.FILANTHROPIC_MODEL || 'sky-apus-4-6',
};

// Current working directory as workspace root
const workspaceRoot = process.cwd();

// Parse args
const args = process.argv.slice(2);

// Special commands
if (args[0] === '--doctor' || args[0] === 'doctor') {
  const { doctor } = require('../lib/doctor');
  doctor();
  process.exit(0);
}

if (args[0] === 'serve' || args[0] === 'ui') {
  const { serve } = require('../lib/server');
  // Parse optional --port flag
  const portIdx = args.indexOf('--port');
  const port = portIdx !== -1 ? parseInt(args[portIdx + 1], 10) : 4321;
  const noBrowser = args.includes('--no-browser');
  serve({ port, noBrowser });
  // server runs until Ctrl-C — do NOT exit here
  return;
}

if (args[0] === '--version' || args[0] === '-v') {
  const pkg = require('../package.json');
  console.log(`Sky-Code v${pkg.version}`);
  process.exit(0);
}

if (args[0] === '--help' || args[0] === '-h' || args.length === 0) {
  console.log(`
🌌 Sky-Code - Offline AI Coding Agent

Usage:
  skycode [command] [options]

Commands:
  prompt <text>       Run a single prompt
  serve               Start the web UI at http://localhost:4321
  doctor              Check system health
  --version, -v       Show version
  --help, -h          Show this help

Options:
  --permission-mode <mode>    Permission mode (read-only, workspace-write, danger-full-access)
  --port <number>             Port for 'serve' command (default: 4321)
  --no-browser                Don't open browser automatically with 'serve'

Examples:
  skycode prompt "What is 2+2?"
  skycode serve
  skycode serve --port 8080
  skycode --permission-mode workspace-write
  skycode doctor

Environment Variables:
  FILANTHROPIC_BASE_URL       API endpoint (default: http://localhost:4000)
  FILANTHROPIC_API_KEY        API key (default: ollama)
  FILANTHROPIC_MODEL          Model name (default: llama3.1:8b)

Documentation: https://github.com/sky-code/sky-code#readme
  `);
  process.exit(0);
}

// Default mode: workspace-write (safe for most use cases)
const permissionMode = '--permission-mode';
const hasPermission = args.some(arg => arg.startsWith(permissionMode));
if (!hasPermission && !args.includes('prompt')) {
  args.unshift('workspace-write');
  args.unshift(permissionMode);
}

function isLocalBridgeUrl(url) {
  return url === 'http://localhost:4000' || url === 'http://127.0.0.1:4000';
}

function probeBridge(timeoutMs = 1200) {
  return new Promise((resolve) => {
    const req = http.get('http://localhost:4000/health', { timeout: timeoutMs }, (res) => {
      // 200/404/405 are all acceptable signals that the bridge HTTP server is alive.
      resolve(res.statusCode === 200 || res.statusCode === 404 || res.statusCode === 405);
      res.resume();
    });

    req.on('error', () => resolve(false));
    req.on('timeout', () => {
      req.destroy();
      resolve(false);
    });
  });
}

function probeBridgeInference(timeoutMs = 25000) {  // Increased for legacy Ollama 0.20.x
  return new Promise((resolve) => {
    const body = JSON.stringify({
      model: env.FILANTHROPIC_MODEL,
      messages: [{ role: 'user', content: 'Reply with OK' }],
      stream: false,
      max_tokens: 4,
    });

    const req = http.request(
      {
        hostname: 'localhost',
        port: 4000,
        path: '/v1/messages',
        method: 'POST',
        timeout: timeoutMs,
        headers: {
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(body),
        },
      },
      (res) => {
        // Any non-5xx response indicates bridge+backend pipeline is alive.
        const ok = res.statusCode && res.statusCode < 500;
        res.resume();
        resolve(Boolean(ok));
      }
    );

    req.on('error', () => resolve(false));
    req.on('timeout', () => {
      req.destroy();
      resolve(false);
    });

    req.write(body);
    req.end();
  });
}

function killBridgeProcess() {
  if (platform === 'win32') {
    spawnSync('taskkill', ['/F', '/IM', 'skybridge.exe'], { stdio: 'ignore' });
    return;
  }
  spawnSync('pkill', ['-f', 'skybridge'], { stdio: 'ignore' });
}

function startBridgeProcess() {
  const bridgeEnv = {
    ...env,
    OLLAMA_BASE_URL: process.env.OLLAMA_BASE_URL || 'http://localhost:11434',
  };

  const child = spawn(skybridgePath, {
    detached: true,
    stdio: 'ignore',
    cwd: workspaceRoot,
    env: bridgeEnv,
  });
  child.unref();
}

async function ensureBridgeReady() {
  if (!isLocalBridgeUrl(env.FILANTHROPIC_BASE_URL)) {
    return true;
  }

  const healthy = await probeBridge();
  if (healthy) {
    const inferenceOk = await probeBridgeInference();
    if (inferenceOk) {
      return true;
    }
  }

  // If bridge is stuck or dead, force-restart it and wait until ready.
  killBridgeProcess();
  startBridgeProcess();

  const maxAttempts = 2;
  for (let i = 0; i < maxAttempts; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 400));
    const isUp = await probeBridge();
    if (!isUp) {
      continue;
    }

    const inferenceOk = await probeBridgeInference();
    if (inferenceOk) {
      return true;
    }
  }

  return false;
}

async function main() {
  const bridgeReady = await ensureBridgeReady();
  if (!bridgeReady) {
    console.error('❌ SkyBridge is not responding on http://localhost:4000');
    console.error('Troubleshooting:');
    console.error(`  1. Run bridge manually: "${skybridgePath}"`);
    console.error('  2. Ensure Ollama is running: ollama serve');
    console.error('  3. Retry: skycode --doctor');
    process.exit(1);
  }

  // Spawn sky binary
  const sky = spawn(skyPath, args, {
    cwd: workspaceRoot,
    stdio: 'inherit',
    env: env
  });

  sky.on('exit', (code) => {
    process.exit(code || 0);
  });

  sky.on('error', (err) => {
    console.error('❌ Failed to start Sky-Code:', err.message);
    console.error('\nTroubleshooting:');
    console.error('  1. Run: skycode --doctor');
    console.error('  2. Ensure Ollama is running: ollama serve');
    console.error('  3. Check GitHub issues: https://github.com/sky-code/sky-code/issues');
    process.exit(1);
  });
}

main().catch((err) => {
  console.error('❌ Unexpected launcher error:', err.message);
  process.exit(1);
});
