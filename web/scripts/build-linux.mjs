import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const repoRoot = path.resolve(__dirname, "..", "..");
const webDir = path.resolve(repoRoot, "web");
const serviceDir = path.resolve(repoRoot, "service");
const serviceEnvPath = path.resolve(serviceDir, ".env");

const run = (cmd, args, opts = {}) =>
  new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: "inherit", shell: false, ...opts });
    p.on("exit", (code) =>
      code === 0
        ? resolve()
        : reject(
            new Error(`${cmd} ${args.join(" ")} exited with code ${code}`),
          ),
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

const generateMinimalServiceFile = (port) => `[Unit]
Description=Framework Control Service
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/framework-control
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
`;

async function main() {
  console.log("[build-linux] Building Framework Control for Linux\n");

  // Collect configuration from env > .env
  const env = process.env;
  const dot = readDotEnv(serviceEnvPath);

  const config = {
    port: env.FRAMEWORK_CONTROL_PORT ?? dot.FRAMEWORK_CONTROL_PORT,
    token: env.FRAMEWORK_CONTROL_TOKEN ?? dot.FRAMEWORK_CONTROL_TOKEN ?? "",
    allowedOrigins:
      env.FRAMEWORK_CONTROL_ALLOWED_ORIGINS ??
      dot.FRAMEWORK_CONTROL_ALLOWED_ORIGINS ??
      "",
    updateRepo:
      env.FRAMEWORK_CONTROL_UPDATE_REPO ??
      dot.FRAMEWORK_CONTROL_UPDATE_REPO ??
      "",
  };

  // Validate required config
  if (!config.port) {
    throw new Error(
      "FRAMEWORK_CONTROL_PORT is required (via env var or service/.env)",
    );
  }
  if (!isValidPort(config.port)) {
    throw new Error(`Invalid port: ${config.port}`);
  }

  console.log("[build-linux] Configuration:");
  console.log(`  Port: ${config.port}`);
  console.log(`  Token: ${config.token ? "***" : "(not set)"}`);
  console.log(`  Allowed Origins: ${config.allowedOrigins || "(none)"}`);
  console.log(`  Update Repo: ${config.updateRepo || "(none)"}`);
  console.log();

  // Build web UI
  console.log("[build-linux] Building web UI...");
  await run("npm", ["run", "build"], { cwd: webDir });

  // Build service binary with baked-in config
  console.log("[build-linux] Building service binary with embedded config...");
  await run("cargo", ["build", "--release", "--features", "embed-ui"], {
    cwd: serviceDir,
    env: {
      ...process.env,
      FRAMEWORK_CONTROL_PORT: config.port,
      FRAMEWORK_CONTROL_TOKEN: config.token,
      FRAMEWORK_CONTROL_ALLOWED_ORIGINS: config.allowedOrigins,
      FRAMEWORK_CONTROL_UPDATE_REPO: config.updateRepo,
    },
  });

  // Prepare distribution
  const distDir = path.resolve(serviceDir, "target", "dist-linux");
  if (fs.existsSync(distDir)) {
    fs.rmSync(distDir, { recursive: true });
  }
  fs.mkdirSync(distDir, { recursive: true });

  // Copy binary
  const binarySource = path.resolve(
    serviceDir,
    "target",
    "release",
    "framework-control-service",
  );
  const binaryDest = path.resolve(distDir, "framework-control");
  fs.copyFileSync(binarySource, binaryDest);
  fs.chmodSync(binaryDest, 0o755);

  // Generate minimal service file (for reference)
  const serviceFile = path.resolve(distDir, "framework-control.service");
  fs.writeFileSync(
    serviceFile,
    generateMinimalServiceFile(config.port),
    "utf8",
  );

  // Create tarball
  const version = process.env.npm_package_version || "0.0.0";
  const tarballName = `framework-control-${version}-linux-x86_64.tar.gz`;
  const tarballPath = path.resolve(serviceDir, "target", tarballName);

  console.log("\n[build-linux] Creating release tarball...");
  await run("tar", [
    "-czf",
    tarballPath,
    "-C",
    distDir,
    "framework-control",
    "framework-control.service",
  ]);

  const binarySize = (fs.statSync(binaryDest).size / 1024 / 1024).toFixed(2);

  console.log("\n✓ Build complete!");
  console.log(`\nBinary: ${binaryDest} (${binarySize} MB)`);
  console.log(`Tarball: ${tarballPath}`);
  console.log(`\nConfiguration is baked into the binary.`);
  console.log(`To run: sudo ./framework-control`);
  console.log(`Listening on: http://127.0.0.1:${config.port}`);
}

main().catch((e) => {
  console.error("\n[build-linux] Build failed:", e?.message || e);
  process.exit(1);
});
