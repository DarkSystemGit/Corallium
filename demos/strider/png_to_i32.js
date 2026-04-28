const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const PNG_SIGNATURE = Buffer.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
]);

function paethPredictor(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

function decodePngRgba(filePath) {
  const data = fs.readFileSync(filePath);
  if (!data.subarray(0, 8).equals(PNG_SIGNATURE)) {
    throw new Error(`Not a PNG file: ${filePath}`);
  }

  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let interlaceMethod = 0;
  const idat = [];

  while (offset + 8 <= data.length) {
    const length = data.readUInt32BE(offset);
    offset += 4;
    const type = data.toString("ascii", offset, offset + 4);
    offset += 4;
    const chunkData = data.subarray(offset, offset + length);
    offset += length;
    offset += 4; // CRC

    if (type === "IHDR") {
      width = chunkData.readUInt32BE(0);
      height = chunkData.readUInt32BE(4);
      bitDepth = chunkData[8];
      colorType = chunkData[9];
      interlaceMethod = chunkData[12];
    } else if (type === "IDAT") {
      idat.push(chunkData);
    } else if (type === "IEND") {
      break;
    }
  }

  if (bitDepth !== 8 || colorType !== 6) {
    throw new Error(
      `Unsupported PNG format in ${filePath}. Expected RGBA8 (bitDepth=8, colorType=6), got bitDepth=${bitDepth}, colorType=${colorType}.`,
    );
  }
  if (interlaceMethod !== 0) {
    throw new Error(`Interlaced PNG is not supported: ${filePath}`);
  }

  const bytesPerPixel = 4;
  const stride = width * bytesPerPixel;
  const inflated = zlib.inflateSync(Buffer.concat(idat));
  const expected = height * (stride + 1);
  if (inflated.length !== expected) {
    throw new Error(
      `Unexpected decoded data size in ${filePath}: expected ${expected} bytes, got ${inflated.length} bytes.`,
    );
  }

  const rgba = Buffer.alloc(width * height * bytesPerPixel);
  for (let y = 0; y < height; y++) {
    const rowStart = y * (stride + 1);
    const filterType = inflated[rowStart];
    const rawRow = inflated.subarray(rowStart + 1, rowStart + 1 + stride);
    const outRowOffset = y * stride;
    const prevRowOffset = (y - 1) * stride;

    for (let x = 0; x < stride; x++) {
      const left =
        x >= bytesPerPixel ? rgba[outRowOffset + x - bytesPerPixel] : 0;
      const up = y > 0 ? rgba[prevRowOffset + x] : 0;
      const upLeft =
        y > 0 && x >= bytesPerPixel
          ? rgba[prevRowOffset + x - bytesPerPixel]
          : 0;

      let value = rawRow[x];
      if (filterType === 1) {
        value = (value + left) & 0xff;
      } else if (filterType === 2) {
        value = (value + up) & 0xff;
      } else if (filterType === 3) {
        value = (value + ((left + up) >> 1)) & 0xff;
      } else if (filterType === 4) {
        value = (value + paethPredictor(left, up, upLeft)) & 0xff;
      } else if (filterType !== 0) {
        throw new Error(
          `Unsupported PNG filter type ${filterType} at row ${y}`,
        );
      }

      rgba[outRowOffset + x] = value;
    }
  }

  return { width, height, rgba };
}

function rgbaToBitmapBytes(width, height, rgba) {
  const pixelCount = width * height;
  const out = Buffer.alloc(pixelCount * 4);
  for (let i = 0; i < pixelCount; i++) {
    const base = i * 4;
    const r = rgba[base];
    const g = rgba[base + 1];
    const b = rgba[base + 2];
    const a = rgba[base + 3];
    // VM color format is 0xRRGGBBAA (stored little-endian in memory/file)
    const packed = ((r << 24) | (g << 16) | (b << 8) | a) >>> 0;
    out.writeUInt32LE(packed, base);
  }
  return out;
}

function main() {
  const defaultInput = path.join(__dirname, "art", "title.png");
  const inputPath = path.resolve(process.argv[2] || defaultInput);
  const defaultOutput = path.join(
    __dirname,
    "linked",

    `${path.basename(inputPath, path.extname(inputPath))}.bitmap`,
  );
  const outputPath = path.resolve(process.argv[3] || defaultOutput);

  const { width, height, rgba } = decodePngRgba(inputPath);
  const output = rgbaToBitmapBytes(width, height, rgba);

  fs.writeFileSync(outputPath, output);
  console.log(
    `Exported ${width}x${height} bitmap (${output.length} bytes) from ${path.relative(process.cwd(), inputPath)} -> ${path.relative(process.cwd(), outputPath)}`,
  );
}

main();
