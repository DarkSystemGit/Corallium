const fs = require("fs");
const path = require("path");

const ROOT_DIR = "./assets";
const OUTPUT_FILE = "atlas.bin";

function getFilesRecursive(dir, fileList = []) {
  const files = fs.readdirSync(dir);
  files.forEach((file) => {
    const filePath = path.join(dir, file);
    if (fs.statSync(filePath).isDirectory()) {
      getFilesRecursive(filePath, fileList);
    } else if (file.endsWith(".tile")) {
      fileList.push(filePath);
    }
  });
  return fileList;
}

/**
 * Converts ARGB buffer (from our previous step) to RGBA.
 * Assuming current tile format in .tile is [B, G, R, A] (Little Endian ARGB)
 * and we want [R, G, B, A].
 */
const PIXSIZE = 4;
function convertToRGBA(buffer) {
  const rgbaBuffer = Buffer.alloc(64 * PIXSIZE);
  for (let i = 0; i < 64 * PIXSIZE; i += PIXSIZE) {
    const g = buffer[i];
    const b = buffer[i + 1];
    const a = buffer[i + 2];
    const r = buffer[i + 3];
    rgbaBuffer[i] = r;
    rgbaBuffer[i + 1] = g;
    rgbaBuffer[i + 2] = b;
    rgbaBuffer[i + 3] = a;
  }
  return rgbaBuffer;
}

function bundleTiles() {
  let tilePaths = getFilesRecursive(ROOT_DIR);
  tilePaths.sort();

  // The count is now original tiles + 1 (for the zero tile)
  const totalTiles = tilePaths.length + 1;
  const totalSize = 2 + totalTiles * (64 * PIXSIZE);
  const atlasBuffer = Buffer.alloc(totalSize);

  // 1. Write Header: Total count
  atlasBuffer.writeInt16LE(totalTiles, 0);

  // 2. Insert the "Zero Tile" at Index 0 (Offset 2)
  // Buffer.alloc defaults to 0, so we just skip these 256 bytes.
  console.log(`[ID 0] Created: Null/Transparent Tile`);

  // 3. Copy and Convert the rest
  tilePaths.forEach((filePath, index) => {
    let tileData = fs.readFileSync(filePath);

    // Ensure we have exactly 256 bytes
    if (tileData.length !== 64 * PIXSIZE) {
      const temp = Buffer.alloc(64 * PIXSIZE);
      tileData.copy(temp);
      tileData = temp;
    }

    // Convert to RGBA
    const rgbaData = convertToRGBA(tileData);

    // Offset: Header(2) + ZeroTile(256) + (CurrentIndex * 256)
    const offset = 2 + 64 * PIXSIZE + index * (64 * PIXSIZE);

    rgbaData.copy(atlasBuffer, offset);
    console.log(`[ID ${index + 1}] Included & Converted: ${filePath}`);
  });

  fs.writeFileSync(OUTPUT_FILE, atlasBuffer);
  console.log(`\nSuccess! Atlas generated with ${totalTiles} tiles (RGBA).`);
}

bundleTiles();
