const fs = require("fs");
const path = require("path");
const https = require("https");

const pkg = require("../package.json");

const assetByPlatform = {
  win32: "sky.exe",
  linux: "sky-linux",
  darwin: "sky-macos"
};

const platform = process.platform;
const asset = assetByPlatform[platform];

if (!asset) {
  console.log(`skycode: unsupported platform ${platform}, skipping binary download.`);
  process.exit(0);
}

const outDir = path.join(__dirname, "bin");
const outName = platform === "win32" ? "sky.exe" : "sky";
const outPath = path.join(outDir, outName);

const versionUrl = `https://github.com/wannabexaker/skycode/releases/download/v${pkg.version}/${asset}`;
const latestUrl = `https://github.com/wannabexaker/skycode/releases/latest/download/${asset}`;

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        response.resume();
        return resolve(download(response.headers.location, destination));
      }

      if (response.statusCode !== 200) {
        response.resume();
        return reject(new Error(`HTTP ${response.statusCode} from ${url}`));
      }

      fs.mkdirSync(path.dirname(destination), { recursive: true });
      const file = fs.createWriteStream(destination);
      response.pipe(file);

      file.on("finish", () => {
        file.close(() => resolve());
      });

      file.on("error", (error) => {
        fs.rmSync(destination, { force: true });
        reject(error);
      });
    });

    request.on("error", reject);
    request.setTimeout(30000, () => {
      request.destroy(new Error("Download timed out"));
    });
  });
}

(async () => {
  if (process.env.SKYCODE_SKIP_DOWNLOAD === "1") {
    console.log("skycode: SKYCODE_SKIP_DOWNLOAD=1, skipping binary download.");
    return;
  }

  try {
    await download(versionUrl, outPath);
    console.log(`skycode: downloaded ${asset} for v${pkg.version}`);
  } catch (error) {
    console.log(`skycode: version asset missing (${error.message}), trying latest release...`);
    await download(latestUrl, outPath);
    console.log(`skycode: downloaded ${asset} from latest release`);
  }

  if (platform !== "win32") {
    fs.chmodSync(outPath, 0o755);
  }
})();
