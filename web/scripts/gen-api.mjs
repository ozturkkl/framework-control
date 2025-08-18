import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import fs from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const serviceDir = path.resolve(__dirname, '..', '..', 'service');
const openapiPath = path.resolve(__dirname, '..', 'openapi.json');
const clientOutDir = path.resolve(__dirname, '..', 'src', 'api');

console.log(`[gen-api] serviceDir=${serviceDir}`);
console.log(`[gen-api] clientOutDir=${clientOutDir}`);
const isolatedTargetDir = path.resolve(serviceDir, 'target', 'openapi');

function run(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: 'inherit', ...opts });
    p.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} ${args.join(' ')} -> ${code}`))));
    p.on('error', reject);
  });
}

// 1) Generate OpenAPI by running service with flag in an isolated target dir to avoid locking conflicts
await run('cargo', ['run', '--', '--generate-openapi'], {
  cwd: serviceDir,
  env: { ...process.env, CARGO_TARGET_DIR: isolatedTargetDir },
});
if (!fs.existsSync(openapiPath)) {
  console.warn(`[gen-api] openapi.json not found at ${openapiPath}`);
  process.exit(0);
}

// 2) Generate TS API client from OpenAPI
console.log(`[gen-api] found openapi spec at ${openapiPath}`);
try {
  fs.mkdirSync(clientOutDir, { recursive: true });
  const { generate } = await import('openapi-typescript-codegen');
  await generate({
    input: openapiPath,
    output: clientOutDir,
    httpClient: 'fetch',
    skipValidateSpec: true,
    useUnionTypes: true,
  });
  console.log(`[gen-api] wrote client to ${clientOutDir}`);
} catch (e) {
  console.warn('[gen-api] openapi client generation skipped:', e?.message || e);
}


