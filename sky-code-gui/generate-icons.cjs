// Generate all icon sizes from the SVG
const sharp = require("sharp");
const fs = require("fs");
const path = require("path");

const svgPath = path.join(__dirname, "src-tauri", "icons", "skycode-icon.svg");
const iconsDir = path.join(__dirname, "src-tauri", "icons");
const svg = fs.readFileSync(svgPath);

const sizes = [
  { name: "32x32.png", w: 32 },
  { name: "64x64.png", w: 64 },
  { name: "128x128.png", w: 128 },
  { name: "128x128@2x.png", w: 256 },
  { name: "icon.png", w: 512 },
  // Square sizes for Windows Store
  { name: "Square30x30Logo.png", w: 30 },
  { name: "Square44x44Logo.png", w: 44 },
  { name: "Square71x71Logo.png", w: 71 },
  { name: "Square89x89Logo.png", w: 89 },
  { name: "Square107x107Logo.png", w: 107 },
  { name: "Square142x142Logo.png", w: 142 },
  { name: "Square150x150Logo.png", w: 150 },
  { name: "Square284x284Logo.png", w: 284 },
  { name: "Square310x310Logo.png", w: 310 },
  { name: "StoreLogo.png", w: 50 },
];

async function generate() {
  for (const { name, w } of sizes) {
    const out = path.join(iconsDir, name);
    await sharp(svg, { density: Math.max(150, Math.round(150 * w / 128)) })
      .resize(w, w)
      .png()
      .toFile(out);
    console.log(`  ✓ ${name} (${w}x${w})`);
  }

  // Generate .ico (256, 64, 48, 32, 16 embedded)
  const icoSizes = [256, 64, 48, 32, 16];
  const buffers = [];
  for (const s of icoSizes) {
    const buf = await sharp(svg, { density: Math.round(150 * s / 128) })
      .resize(s, s)
      .png()
      .toBuffer();
    buffers.push({ size: s, data: buf });
  }

  // Build ICO file manually
  const ico = buildIco(buffers);
  fs.writeFileSync(path.join(iconsDir, "icon.ico"), ico);
  console.log("  ✓ icon.ico (multi-size)");

  console.log("\nAll icons generated!");
}

function buildIco(images) {
  const numImages = images.length;
  const headerSize = 6;
  const dirEntrySize = 16;
  const dirSize = dirEntrySize * numImages;
  let offset = headerSize + dirSize;

  // ICO header
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0);        // Reserved
  header.writeUInt16LE(1, 2);        // ICO type
  header.writeUInt16LE(numImages, 4); // Count

  const dirEntries = [];
  const imageData = [];

  for (const { size, data } of images) {
    const entry = Buffer.alloc(dirEntrySize);
    entry.writeUInt8(size >= 256 ? 0 : size, 0); // width
    entry.writeUInt8(size >= 256 ? 0 : size, 1); // height
    entry.writeUInt8(0, 2);                        // palette
    entry.writeUInt8(0, 3);                        // reserved
    entry.writeUInt16LE(1, 4);                     // color planes
    entry.writeUInt16LE(32, 6);                    // bits per pixel
    entry.writeUInt32LE(data.length, 8);           // size
    entry.writeUInt32LE(offset, 12);               // offset
    dirEntries.push(entry);
    imageData.push(data);
    offset += data.length;
  }

  return Buffer.concat([header, ...dirEntries, ...imageData]);
}

generate().catch(err => { console.error(err); process.exit(1); });
