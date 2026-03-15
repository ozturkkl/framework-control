import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const repoRoot = resolve(__dirname, '..', '..');

const newVersion = process.argv[2];
if (!newVersion) {
	console.error('Usage: node scripts/bump-version.mjs <new-version>');
	process.exit(1);
}

// Update web/package.json
const packageJsonPath = resolve(repoRoot, 'web', 'package.json');
const pkg = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
pkg.version = newVersion;
writeFileSync(packageJsonPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`Updated web/package.json`);

// Update service/Cargo.toml (only the [package] version line, not dependency versions)
const cargoTomlPath = resolve(repoRoot, 'service', 'Cargo.toml');
const cargoToml = readFileSync(cargoTomlPath, 'utf8');
const updatedCargoToml = cargoToml.replace(/^version = ".*"/m, `version = "${newVersion}"`);
writeFileSync(cargoTomlPath, updatedCargoToml);
console.log(`Updated service/Cargo.toml`);

// Update nix/package.nix if it exists
const nixPackagePath = resolve(repoRoot, 'nix', 'package.nix');
if (existsSync(nixPackagePath)) {
	const nixPackage = readFileSync(nixPackagePath, 'utf8');
	const updatedNixPackage = nixPackage.replace(/^(\s*version = )".*";/m, `$1"${newVersion}";`);
	writeFileSync(nixPackagePath, updatedNixPackage);
	console.log(`Updated nix/package.nix`);
}

// Sync lock files
console.log(`Syncing web/package-lock.json...`);
execSync('npm install', { cwd: resolve(repoRoot, 'web'), stdio: 'inherit' });

console.log(`Syncing service/Cargo.lock...`);
execSync('cargo fetch', { cwd: resolve(repoRoot, 'service'), stdio: 'inherit' });

console.log(`\nBumped to ${newVersion}`);
