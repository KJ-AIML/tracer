import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const dir = dirname(fileURLToPath(import.meta.url));

function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return ~c >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeB = Buffer.from(type);
  const crcB = Buffer.alloc(4);
  crcB.writeUInt32BE(crc32(Buffer.concat([typeB, data])));
  return Buffer.concat([len, typeB, data, crcB]);
}

function makePng(w, h) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const i = y * (w * 4 + 1) + 1 + x * 4;
      raw[i] = 15;
      raw[i + 1] = 18;
      raw[i + 2] = 24;
      raw[i + 3] = 255;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Classic BMP-in-ICO compatible with Windows RC.EXE */
function makeBmpIco(size) {
  const headerSize = 40;
  const xorSize = size * size * 4;
  const andRow = Math.ceil(size / 32) * 4;
  const andSize = andRow * size;
  const imageSize = headerSize + xorSize + andSize;

  const bmp = Buffer.alloc(imageSize);
  bmp.writeUInt32LE(40, 0);
  bmp.writeInt32LE(size, 4);
  bmp.writeInt32LE(size * 2, 8);
  bmp.writeUInt16LE(1, 12);
  bmp.writeUInt16LE(32, 14);
  bmp.writeUInt32LE(0, 16);
  bmp.writeUInt32LE(xorSize + andSize, 20);

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const destY = size - 1 - y;
      const i = headerSize + (destY * size + x) * 4;
      bmp[i] = 24;
      bmp[i + 1] = 18;
      bmp[i + 2] = 15;
      bmp[i + 3] = 255;
    }
  }

  const ico = Buffer.alloc(6 + 16 + imageSize);
  ico.writeUInt16LE(0, 0);
  ico.writeUInt16LE(1, 2);
  ico.writeUInt16LE(1, 4);
  ico[6] = size >= 256 ? 0 : size;
  ico[7] = size >= 256 ? 0 : size;
  ico[8] = 0;
  ico[9] = 0;
  ico.writeUInt16LE(1, 10);
  ico.writeUInt16LE(32, 12);
  ico.writeUInt32LE(imageSize, 14);
  ico.writeUInt32LE(22, 18);
  bmp.copy(ico, 22);
  return ico;
}

const png32 = makePng(32, 32);
const png128 = makePng(128, 128);
const ico = makeBmpIco(32);

writeFileSync(join(dir, "icon.png"), png32);
writeFileSync(join(dir, "32x32.png"), png32);
writeFileSync(join(dir, "128x128.png"), png128);
writeFileSync(join(dir, "icon.ico"), ico);
console.log("icons written", { png32: png32.length, ico: ico.length });
