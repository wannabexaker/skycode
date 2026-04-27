const { execSync } = require('child_process');
const chalk = require('chalk');
const which = require('which');
const http = require('http');
const path = require('path');
const os = require('os');
const fs = require('fs');

function doctor() {
  console.log(chalk.cyan('\n🏥 Sky-Code Health Check\n'));

  const checks = {
    node: false,
    ollama: false,
    ollamaRunning: false,
    model: false,
    skybridge: false,
    binary: false
  };

  // Check Node.js
  try {
    const nodeVersion = process.version;
    const major = parseInt(nodeVersion.slice(1).split('.')[0]);
    if (major >= 18) {
      console.log(chalk.green('✓'), 'Node.js:', nodeVersion);
      checks.node = true;
    } else {
      console.log(chalk.red('✗'), 'Node.js:', nodeVersion, '(need >=18.0.0)');
    }
  } catch (err) {
    console.log(chalk.red('✗'), 'Node.js: Not found');
  }

  // Check Sky-Code + SkyBridge binaries
  const platform = os.platform();
  const arch = os.arch();
  const platformMap = {
    'win32': 'win-x64',
    'linux': 'linux-x64',
    'darwin': arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64'
  };
  const binaryDir = path.join(__dirname, '..', 'binaries', platformMap[platform]);
  const skyBinary = platform === 'win32' ? 'sky.exe' : 'sky';
  const bridgeBinary = platform === 'win32' ? 'skybridge.exe' : 'skybridge';
  const skyPath = path.join(binaryDir, skyBinary);
  const bridgePath = path.join(binaryDir, bridgeBinary);

  if (fs.existsSync(skyPath)) {
    const stats = fs.statSync(skyPath);
    const sizeMB = (stats.size / 1024 / 1024).toFixed(1);
    console.log(chalk.green('✓'), `Sky-Code binary: ${sizeMB} MB (${platformMap[platform]})`);
    checks.binary = true;
  } else {
    console.log(chalk.red('✗'), `Sky-Code binary: Not found at ${skyPath}`);
  }

  if (fs.existsSync(bridgePath)) {
    const stats = fs.statSync(bridgePath);
    const sizeMB = (stats.size / 1024 / 1024).toFixed(1);
    console.log(chalk.green('✓'), `SkyBridge binary: ${sizeMB} MB (${platformMap[platform]})`);
  } else {
    console.log(chalk.red('✗'), `SkyBridge binary: Not found at ${bridgePath}`);
    checks.binary = false;
  }

  // Check Ollama
  try {
    which.sync('ollama');
    const ollamaVersion = execSync('ollama --version', { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }).trim();
    console.log(chalk.green('✓'), 'Ollama installed:', ollamaVersion);
    checks.ollama = true;
  } catch (err) {
    console.log(chalk.red('✗'), 'Ollama: Not found');
    console.log(chalk.yellow('  Install: https://ollama.com/download'));
  }

  // Check if Ollama is running
  if (checks.ollama) {
    try {
      execSync('ollama list', { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'], timeout: 3000 });
      console.log(chalk.green('✓'), 'Ollama service: Running');
      checks.ollamaRunning = true;
    } catch (err) {
      console.log(chalk.yellow('⚠'), 'Ollama service: Not running');
      console.log(chalk.yellow('  Start: ollama serve'));
    }
  }

  // Check Model
  if (checks.ollamaRunning) {
    try {
      const models = execSync('ollama list', { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] });
      if (models.includes('llama3.1:8b')) {
        console.log(chalk.green('✓'), 'Model backend: llama3.1:8b (for sky-apus/sky-sannet/sky-haiku aliases)');
        checks.model = true;
      } else {
        console.log(chalk.yellow('⚠'), 'Model: llama3.1:8b not found');
        console.log(chalk.yellow('  Run: ollama pull llama3.1:8b'));
        console.log(chalk.dim('      (This will download ~4.9 GB)'));
      }
    } catch (err) {
      console.log(chalk.red('✗'), 'Model check failed');
    }
  }

  // Check SkyBridge (optional)
  return new Promise((resolve) => {
    const req = http.get('http://localhost:4000/health', { timeout: 2000 }, (res) => {
      if (res.statusCode === 200 || res.statusCode === 404 || res.statusCode === 405) {
        console.log(chalk.green('✓'), 'SkyBridge: Running on port 4000');
        checks.skybridge = true;
      } else {
        console.log(chalk.yellow('⚠'), 'SkyBridge: Not running (optional, auto-starts if needed)');
      }
      finishCheck();
    });

    req.on('error', () => {
      console.log(chalk.yellow('⚠'), 'SkyBridge: Not running (optional, auto-starts if needed)');
      finishCheck();
    });

    req.on('timeout', () => {
      req.destroy();
      console.log(chalk.yellow('⚠'), 'SkyBridge: Not running (optional)');
      finishCheck();
    });

    function finishCheck() {
      // Summary
      const healthy = checks.node && checks.binary && checks.ollama && checks.ollamaRunning && checks.model;
      console.log('\n' + chalk.bold('Health Status:'), healthy ? chalk.green('HEALTHY ✓') : chalk.red('UNHEALTHY ✗'));

      if (!healthy) {
        console.log(chalk.yellow('\n⚠ Action Required:'));
        if (!checks.node) console.log('  • Update Node.js to v18+: https://nodejs.org/');
        if (!checks.binary) console.log('  • Reinstall: npm install -g sky-code');
        if (!checks.ollama) console.log('  • Install Ollama: https://ollama.com/download');
        if (!checks.ollamaRunning) console.log('  • Start Ollama: ollama serve');
        if (!checks.model) console.log('  • Pull model: ollama pull llama3.1:8b');
        console.log('\nThen run: ' + chalk.cyan('skycode --doctor') + '\n');
      } else {
        console.log(chalk.green('\n🎉 All systems ready! Try:\n'));
        console.log('  ' + chalk.cyan('skycode prompt "What is 2+2?"'));
        console.log('  ' + chalk.cyan('skycode') + '  # Interactive mode\n');
      }

      resolve({ healthy, checks });
    }
  });
}

module.exports = { doctor };

// CLI entry point
if (require.main === module) {
  doctor().then(({ healthy }) => {
    process.exit(healthy ? 0 : 1);
  });
}
