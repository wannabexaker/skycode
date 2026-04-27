const fs = require('fs');
const path = require('path');
const os = require('os');
const chalk = require('chalk');

console.log(chalk.cyan('\n📦 Installing Sky-Code...\n'));

const platform = os.platform();
const arch = os.arch();

const platformMap = {
  'win32': 'win-x64',
  'linux': 'linux-x64',
  'darwin': arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64'
};

const binaryPlatform = platformMap[platform];
if (!binaryPlatform) {
  console.error(chalk.red('✗'), `Unsupported platform: ${platform} ${arch}`);
  console.error('Supported: Windows x64, Linux x64, macOS x64/arm64');
  process.exit(1);
}

const binaryDir = path.join(__dirname, '..', 'binaries', binaryPlatform);

// Check if binaries exist
const skyBinary = platform === 'win32' ? 'sky.exe' : 'sky';
const bridgeBinary = platform === 'win32' ? 'skybridge.exe' : 'skybridge';
const skyPath = path.join(binaryDir, skyBinary);
const skybridgePath = path.join(binaryDir, bridgeBinary);

if (!fs.existsSync(skyPath)) {
  console.log(chalk.yellow('⚠'), `Binary not found for ${binaryPlatform}`);
  console.log(chalk.yellow('  This is expected for beta testing.'));
  console.log(chalk.yellow('  Copy binaries manually to: binaries/${binaryPlatform}/'));
} else {
  console.log(chalk.green('✓'), `Found binary for ${binaryPlatform}`);
}

if (!fs.existsSync(skybridgePath)) {
  console.log(chalk.yellow('⚠'), `SkyBridge binary not found for ${binaryPlatform}`);
  console.log(chalk.yellow('  Sky-Code may hang at thinking if bridge is missing.'));
} else {
  console.log(chalk.green('✓'), `Found SkyBridge for ${binaryPlatform}`);
}

// Set executable permissions on Unix
if (platform !== 'win32' && fs.existsSync(skyPath)) {
  try {
    fs.chmodSync(skyPath, 0o755);
    if (fs.existsSync(skybridgePath)) {
      fs.chmodSync(skybridgePath, 0o755);
    }
    console.log(chalk.green('✓'), 'Set executable permissions');
  } catch (err) {
    console.error(chalk.red('✗'), 'Failed to set permissions:', err.message);
    console.error('You may need to run: chmod +x ' + skyPath);
  }
}

console.log(chalk.green('\n✓ Sky-Code installed successfully!\n'));
console.log(chalk.bold('Next steps:\n'));
console.log('  1. Check health: ' + chalk.cyan('skycode --doctor'));
console.log('  2. Ensure Ollama is running: ' + chalk.cyan('ollama serve'));
console.log('  3. Try it: ' + chalk.cyan('skycode prompt "What is 2+2?"'));
console.log('\n' + chalk.dim('Documentation: https://github.com/sky-code/sky-code#readme') + '\n');
