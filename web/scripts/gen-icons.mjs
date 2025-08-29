import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';
import pngToIco from 'png-to-ico';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const webDir = path.resolve(__dirname, '..');
const publicDir = path.resolve(webDir, 'public');
const assetsDir = path.resolve(publicDir, 'assets');
const outDir = path.resolve(assetsDir, 'generated');

async function ensureDir(dir) {
  await fs.promises.mkdir(dir, { recursive: true });
}

async function generate() {
  const source = path.resolve(assetsDir, 'logo.png');
  if (!fs.existsSync(source)) {
    console.error(`[gen-icons] Source not found: ${source}`);
    process.exit(1);
  }
  await ensureDir(outDir);

  const outputs = [
    { file: 'icon-192.png', size: 192 },
    { file: 'icon-512.png', size: 512 },
    { file: 'maskable-icon-192.png', size: 192 },
    { file: 'maskable-icon-512.png', size: 512 },
    { file: 'favicon-32.png', size: 32 },
    { file: 'apple-touch-icon.png', size: 180 },
  ];

  for (const out of outputs) {
    const dest = path.resolve(outDir, out.file);
    await sharp(source)
      .resize(out.size, out.size, { fit: 'cover' })
      .png()
      .toFile(dest);
    console.log(`[gen-icons] Wrote ${path.relative(publicDir, dest)}`);
  }

  // Generate ICO from 256px PNG (best for Windows icons)
  const png256 = await sharp(source).resize(256, 256, { fit: 'cover' }).png().toBuffer();
  const icoBuf = await pngToIco(png256);
  const icoDest = path.resolve(outDir, 'logo.ico');
  await fs.promises.writeFile(icoDest, icoBuf);
  console.log(`[gen-icons] Wrote ${path.relative(publicDir, icoDest)}`);
}

generate().catch((e) => {
  console.error('[gen-icons] Failed:', e?.message || e);
  process.exit(1);
});


