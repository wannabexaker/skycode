#!/usr/bin/env node
/**
 * SkyCode Web Server
 * Serves the SkyCode UI and provides a streaming chat API
 * so the React app works in a normal browser (localhost mode).
 *
 * Usage: skycode serve [--port 4321]
 */

'use strict';

const http       = require('http');
const https      = require('https');
const fs         = require('fs');
const path       = require('path');
const os         = require('os');
const { spawn }  = require('child_process');
const url        = require('url');

// ── Platform binary resolution ─────────────────────────────────────────────
const PLATFORM_MAP = {
  win32:  'win-x64',
  linux:  'linux-x64',
  darwin: os.arch() === 'arm64' ? 'darwin-arm64' : 'darwin-x64',
};

const PLATFORM_KEY   = PLATFORM_MAP[os.platform()] || 'win-x64';
const IS_WIN         = os.platform() === 'win32';
const BIN_DIR        = path.join(__dirname, '..', 'binaries', PLATFORM_KEY);
const SKY_BIN        = path.join(BIN_DIR, IS_WIN ? 'sky.exe'        : 'sky');
const BRIDGE_BIN     = path.join(BIN_DIR, IS_WIN ? 'skybridge.exe'  : 'skybridge');
const UI_DIST        = path.join(__dirname, '..', 'ui');   // pre-built React assets

// Map file extensions → MIME types
const MIME = {
  '.html': 'text/html',
  '.js':   'application/javascript',
  '.mjs':  'application/javascript',
  '.css':  'text/css',
  '.svg':  'image/svg+xml',
  '.png':  'image/png',
  '.ico':  'image/x-icon',
  '.woff2':'font/woff2',
  '.woff': 'font/woff',
  '.ttf':  'font/ttf',
  '.json': 'application/json',
  '.map':  'application/json',
};

// ── SkyBridge lifecycle ────────────────────────────────────────────────────
let bridgeProc = null;
let bridgeReady = false;

function startBridge(env, onReady) {
  if (!fs.existsSync(BRIDGE_BIN)) {
    console.warn('[server] SkyBridge binary not found:', BRIDGE_BIN);
    onReady();
    return;
  }

  console.log('[server] Starting SkyBridge…');
  bridgeProc = spawn(BRIDGE_BIN, [], {
    env: { ...process.env, ...env },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const check = setInterval(() => {
    const req = http.get('http://localhost:4000/health', { timeout: 800 }, (res) => {
      if (!bridgeReady && (res.statusCode === 200 || res.statusCode === 404 || res.statusCode === 405)) {
        clearInterval(check);
        bridgeReady = true;
        console.log('[server] SkyBridge is up on :4000');
        onReady();
      }
      res.resume();
    });
    req.on('error', () => {});
  }, 600);

  bridgeProc.stdout.on('data', (d) => process.stdout.write('[bridge] ' + d));
  bridgeProc.stderr.on('data', (d) => process.stderr.write('[bridge] ' + d));
  bridgeProc.on('exit', (code) => {
    console.warn('[server] SkyBridge exited with code', code);
    bridgeReady = false;
  });
}

// ── Streaming chat session ─────────────────────────────────────────────────
// Each request spawns `sky --permission-mode workspace-write`
// and streams lines as SSE events back to the browser.

function handleChat(req, res, body, env) {
  let parsed;
  try { parsed = JSON.parse(body); } catch {
    res.writeHead(400); res.end('Bad JSON'); return;
  }

  const { message, model } = parsed;
  if (!message || typeof message !== 'string') {
    res.writeHead(400); res.end('Missing message'); return;
  }

  const safeModel = (model && /^[\w.:/-]+$/.test(model)) ? model : 'llama3.1:8b';

  res.writeHead(200, {
    'Content-Type':  'text/event-stream',
    'Cache-Control': 'no-cache',
    'Connection':    'keep-alive',
    'Access-Control-Allow-Origin': '*',
  });

  const chatEnv = {
    ...process.env,
    ...env,
    FILANTHROPIC_MODEL: safeModel,
  };

  if (!fs.existsSync(SKY_BIN)) {
    res.write(`data: ${JSON.stringify({ content: 'sky binary not found: ' + SKY_BIN, is_complete: false })}\n\n`);
    res.write(`data: ${JSON.stringify({ content: '', is_complete: true })}\n\n`);
    res.end();
    return;
  }

  const sky = spawn(SKY_BIN, ['--permission-mode', 'workspace-write'], {
    env: chatEnv,
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  sky.stdin.write(message + '\n');
  sky.stdin.end();

  sky.stdout.on('data', (chunk) => {
    const text = chunk.toString();
    const payload = JSON.stringify({ content: text, is_complete: false });
    res.write(`data: ${payload}\n\n`);
  });

  sky.stderr.on('data', (chunk) => {
    // stderr is debug from sky — suppress from SSE unless needed
    process.stderr.write('[sky] ' + chunk);
  });

  sky.on('close', () => {
    res.write(`data: ${JSON.stringify({ content: '', is_complete: true })}\n\n`);
    res.end();
  });

  req.on('close', () => sky.kill());
}

// ── Static file serving ────────────────────────────────────────────────────
function serveStatic(req, res, reqPath) {
  // Sanitise the path to prevent path traversal
  const safePath = path.normalize(reqPath).replace(/^(\.\.[/\\])+/, '');
  let filePath = path.join(UI_DIST, safePath);

  // Default to index.html for SPA routing
  if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
    filePath = path.join(UI_DIST, 'index.html');
  }

  if (!fs.existsSync(filePath)) {
    res.writeHead(404); res.end('Not found'); return;
  }

  const ext  = path.extname(filePath);
  const mime = MIME[ext] || 'application/octet-stream';
  const stat = fs.statSync(filePath);

  res.writeHead(200, {
    'Content-Type':   mime,
    'Content-Length': stat.size,
    'Cache-Control':  ext === '.html' ? 'no-cache' : 'public, max-age=31536000, immutable',
  });
  fs.createReadStream(filePath).pipe(res);
}

// ── HTTP server ────────────────────────────────────────────────────────────
function startServer(port, bridgeEnv) {
  const server = http.createServer((req, res) => {
    const parsedUrl = url.parse(req.url || '/');
    const pathname  = parsedUrl.pathname || '/';

    // CORS preflight
    if (req.method === 'OPTIONS') {
      res.writeHead(204, {
        'Access-Control-Allow-Origin':  '*',
        'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type',
      });
      res.end();
      return;
    }

    // Health endpoint
    if (pathname === '/health') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ status: 'ok', bridge: bridgeReady }));
      return;
    }

    // Chat API (SSE streaming)
    if (pathname === '/api/chat' && req.method === 'POST') {
      let body = '';
      req.on('data', (chunk) => { body += chunk; });
      req.on('end', () => handleChat(req, res, body, bridgeEnv));
      return;
    }

    // Static UI assets
    if (req.method === 'GET') {
      serveStatic(req, res, pathname);
      return;
    }

    res.writeHead(405); res.end('Method not allowed');
  });

  server.listen(port, '127.0.0.1', () => {
    console.log('\n  \x1b[36mSkyCode UI\x1b[0m running at \x1b[1mhttp://localhost:' + port + '\x1b[0m');
    console.log('  Press Ctrl+C to stop\n');
  });

  return server;
}

// ── Open browser ───────────────────────────────────────────────────────────
function openBrowser(url) {
  const cmd = IS_WIN ? 'start' : (os.platform() === 'darwin' ? 'open' : 'xdg-open');
  const { exec } = require('child_process');
  exec(`${cmd} ${url}`);
}

// ── Main entry ─────────────────────────────────────────────────────────────
function serve(opts = {}) {
  const port = opts.port || parseInt(process.env.SKYCODE_UI_PORT || '4321', 10);
  const noBrowser = opts.noBrowser || false;

  const bridgeEnv = {
    FILANTHROPIC_BASE_URL: process.env.FILANTHROPIC_BASE_URL || 'http://localhost:4000',
    FILANTHROPIC_API_KEY:  process.env.FILANTHROPIC_API_KEY  || 'ollama',
    FILANTHROPIC_MODEL:    process.env.FILANTHROPIC_MODEL    || 'llama3.1:8b',
    OLLAMA_HOST:           process.env.OLLAMA_HOST           || 'http://localhost:11434',
  };

  // Check if SkyBridge is already running before starting a new one
  const req = http.get('http://localhost:4000/health', { timeout: 800 }, (res) => {
    bridgeReady = (res.statusCode === 200 || res.statusCode === 404 || res.statusCode === 405);
    res.resume();
    if (bridgeReady) {
      console.log('[server] SkyBridge already running on :4000');
      const s = startServer(port, bridgeEnv);
      if (!noBrowser) openBrowser(`http://localhost:${port}`);
    } else {
      startBridge(bridgeEnv, () => {
        const s = startServer(port, bridgeEnv);
        if (!noBrowser) openBrowser(`http://localhost:${port}`);
      });
    }
  });
  req.on('error', () => {
    startBridge(bridgeEnv, () => {
      const s = startServer(port, bridgeEnv);
      if (!noBrowser) openBrowser(`http://localhost:${port}`);
    });
  });

  // Cleanup on exit
  process.on('SIGINT',  () => { if (bridgeProc) bridgeProc.kill(); process.exit(0); });
  process.on('SIGTERM', () => { if (bridgeProc) bridgeProc.kill(); process.exit(0); });
}

module.exports = { serve };
