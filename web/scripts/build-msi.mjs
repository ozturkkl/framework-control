import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const repoRoot = path.resolve(__dirname, "..", "..");
const webDir = path.resolve(repoRoot, "web");
const serviceDir = path.resolve(repoRoot, "service");
const wixXmlPath = path.resolve(
  serviceDir,
  "wix",
  "FrameworkControlService.xml",
);
const serviceEnvPath = path.resolve(serviceDir, ".env");
const packageJsonPath = path.resolve(webDir, "package.json");

const run = (cmd, args, opts = {}) =>
  new Promise((resolve, reject) => {
    const p = spawn(cmd, args, {
      stdio: "inherit",
      shell: true,
      ...opts,
    });
    p.on("exit", (code) =>
      code === 0
        ? resolve()
        : reject(new Error(`${cmd} ${args.join(" ")} -> ${code}`)),
    );
    p.on("error", reject);
  });

const readDotEnv = (p) => {
  if (!fs.existsSync(p)) return {};
  return Object.fromEntries(
    fs
      .readFileSync(p, "utf8")
      .split(/\r?\n/)
      .map((l) => l.match(/^([A-Z0-9_]+)=(.*)$/))
      .filter(Boolean)
      .map((m) => [m[1], m[2].replace(/^['"]|['"]$/g, "")]),
  );
};

const isValidPort = (v) =>
  Number.isInteger(Number(v)) && Number(v) >= 1 && Number(v) <= 65535;

const replaceTokens = (xml, t) =>
  xml
    .replaceAll("@FRAMEWORK_CONTROL_ALLOWED_ORIGINS@", t.ALLOWED_ORIGINS)
    .replaceAll("@FRAMEWORK_CONTROL_TOKEN@", t.CONTROL_TOKEN)
    .replaceAll("@FRAMEWORK_CONTROL_PORT@", t.CONTROL_PORT)
    .replaceAll("@FRAMEWORK_CONTROL_UPDATE_REPO@", t.UPDATE_REPO);

async function main() {
  console.log("[build-msi] Building Framework Control MSI for Windows\n");

  // Check and install cargo-wix if needed
  console.log("[build-msi] Checking for cargo-wix...");
  try {
    await run("cargo.exe", ["wix", "--version"], { cwd: serviceDir });
    console.log("[build-msi] cargo-wix is already installed\n");
  } catch (e) {
    console.log("[build-msi] cargo-wix not found, installing...");
    await run("cargo.exe", ["install", "cargo-wix"], { cwd: serviceDir });
    console.log("[build-msi] cargo-wix installed successfully\n");
  }

  // Install web dependencies
  console.log("[build-msi] Installing web dependencies...");
  await run("npm.cmd", ["ci"], { cwd: webDir });
  console.log("[build-msi] Web dependencies installed\n");

  // Read version from package.json
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  const version = packageJson.version;
  if (!version) {
    throw new Error("version not found in package.json");
  }
  console.log(`[build-msi] Version: ${version}\n`);

  // Collect configuration from env > .env
  const env = process.env;
  const dot = readDotEnv(serviceEnvPath);

  const config = {
    port: env.FRAMEWORK_CONTROL_PORT ?? dot.FRAMEWORK_CONTROL_PORT,
    token: env.FRAMEWORK_CONTROL_TOKEN ?? dot.FRAMEWORK_CONTROL_TOKEN,
    allowedOrigins:
      env.FRAMEWORK_CONTROL_ALLOWED_ORIGINS ??
      dot.FRAMEWORK_CONTROL_ALLOWED_ORIGINS,
    updateRepo:
      env.FRAMEWORK_CONTROL_UPDATE_REPO ?? dot.FRAMEWORK_CONTROL_UPDATE_REPO,
  };

  // Validate all config upfront
  if (!config.port) {
    throw new Error(
      "FRAMEWORK_CONTROL_PORT is required (via env var or service/.env)",
    );
  }
  if (!isValidPort(config.port)) {
    throw new Error(`Invalid port: ${config.port}`);
  }
  if (config.token === undefined) {
    throw new Error(
      "FRAMEWORK_CONTROL_TOKEN is required (via env var or service/.env)",
    );
  }
  if (config.allowedOrigins === undefined) {
    throw new Error(
      "FRAMEWORK_CONTROL_ALLOWED_ORIGINS is required (via env var or service/.env)",
    );
  }
  if (config.updateRepo === undefined) {
    throw new Error(
      "FRAMEWORK_CONTROL_UPDATE_REPO is required (via env var or service/.env)",
    );
  }

  console.log("[build-msi] Configuration:");
  console.log(`  Port: ${config.port}`);
  console.log(`  Token: ${config.token ? "***" : "(not set)"}`);
  console.log(`  Allowed Origins: ${config.allowedOrigins || "(none)"}`);
  console.log(`  Update Repo: ${config.updateRepo || "(none)"}`);
  console.log();

  // Build web UI (prebuild runs automatically via npm)
  console.log("[build-msi] Building web UI...");
  await run("npm.cmd", ["run", "build"], {
    cwd: webDir,
    env: {
      ...process.env,
      GITHUB_PAGES: env.GITHUB_PAGES,
      VITE_BASE: env.VITE_BASE,
      VITE_API_BASE: env.VITE_API_BASE,
      VITE_CONTROL_TOKEN: env.VITE_CONTROL_TOKEN,
    },
  });

  // Build service binary
  console.log("[build-msi] Building service binary...");
  await run("cargo.exe", ["build", "--release"], { cwd: serviceDir });

  // Prepare tokens for WiX XML
  const tokens = {
    ALLOWED_ORIGINS: config.allowedOrigins,
    CONTROL_TOKEN: config.token,
    CONTROL_PORT: config.port,
    UPDATE_REPO: config.updateRepo,
  };

  // Replace tokens, run wix, restore XML
  console.log("[build-msi] Building MSI with cargo-wix...");
  const original = fs.readFileSync(wixXmlPath, "utf8");
  fs.writeFileSync(wixXmlPath, replaceTokens(original, tokens), "utf8");
  try {
    await run("cargo.exe", ["wix", "--nocapture", "-v"], { cwd: serviceDir });
  } finally {
    fs.writeFileSync(wixXmlPath, original, "utf8");
  }

  // Locate the MSI
  const wixDir = path.resolve(serviceDir, "target", "wix");
  const msiFiles = fs
    .readdirSync(wixDir)
    .filter((f) => f.endsWith(".msi"))
    .sort((a, b) => {
      const statA = fs.statSync(path.resolve(wixDir, a));
      const statB = fs.statSync(path.resolve(wixDir, b));
      return statB.mtimeMs - statA.mtimeMs;
    });

  if (msiFiles.length === 0) {
    throw new Error("MSI not found in target/wix");
  }

  const msiPath = path.resolve(wixDir, msiFiles[0]);
  const msiSize = (fs.statSync(msiPath).size / 1024 / 1024).toFixed(2);

  console.log("\n✓ Build complete!");
  console.log(`\nMSI: ${msiPath} (${msiSize} MB)`);
  console.log(`Version: ${version}`);
  console.log(`\nConfiguration is embedded in the MSI.`);
}

main().catch((e) => {
  console.error("[build-msi] Failed:", e?.message || e);
  process.exit(1);
});
