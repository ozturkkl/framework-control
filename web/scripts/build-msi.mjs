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

const run = (cmd, args, opts = {}) =>
  new Promise((resolve, reject) => {
    const p = spawn(cmd, args, {
      stdio: "inherit",
      shell: process.platform === "win32",
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
  // Build web and service
  await run(
    process.platform === "win32" ? "npm.cmd" : "npm",
    ["run", "build"],
    { cwd: webDir },
  );
  await run(
    process.platform === "win32" ? "cargo.exe" : "cargo",
    ["build", "--release"],
    { cwd: serviceDir },
  );

  // Prepare tokens from env and .env file
  const env = process.env;
  const dot = readDotEnv(serviceEnvPath);
  const tokens = {
    ALLOWED_ORIGINS:
      env.FRAMEWORK_CONTROL_ALLOWED_ORIGINS ??
      dot.FRAMEWORK_CONTROL_ALLOWED_ORIGINS,
    CONTROL_TOKEN: env.FRAMEWORK_CONTROL_TOKEN ?? dot.FRAMEWORK_CONTROL_TOKEN,
    CONTROL_PORT: env.FRAMEWORK_CONTROL_PORT ?? dot.FRAMEWORK_CONTROL_PORT,
    UPDATE_REPO:
      env.FRAMEWORK_CONTROL_UPDATE_REPO ?? dot.FRAMEWORK_CONTROL_UPDATE_REPO,
  };

  // Validate all config upfront
  if (!tokens.CONTROL_PORT || !isValidPort(tokens.CONTROL_PORT)) {
    throw new Error(
      "FRAMEWORK_CONTROL_PORT is required (via env FRAMEWORK_CONTROL_PORT or service/.env)",
    );
  }
  if (tokens.CONTROL_TOKEN === undefined) {
    throw new Error(
      "FRAMEWORK_CONTROL_TOKEN is required (via env FRAMEWORK_CONTROL_TOKEN or service/.env)",
    );
  }
  if (tokens.ALLOWED_ORIGINS === undefined) {
    throw new Error(
      "FRAMEWORK_CONTROL_ALLOWED_ORIGINS is required (via env FRAMEWORK_CONTROL_ALLOWED_ORIGINS or service/.env)",
    );
  }
  if (tokens.UPDATE_REPO === undefined) {
    throw new Error(
      "FRAMEWORK_CONTROL_UPDATE_REPO is required (via env FRAMEWORK_CONTROL_UPDATE_REPO or service/.env)",
    );
  }

  // Replace tokens, run wix, restore XML
  const original = fs.readFileSync(wixXmlPath, "utf8");
  fs.writeFileSync(wixXmlPath, replaceTokens(original, tokens), "utf8");
  try {
    await run(
      process.platform === "win32" ? "cargo.exe" : "cargo",
      ["wix", "--nocapture", "-v"],
      { cwd: serviceDir },
    );
  } finally {
    fs.writeFileSync(wixXmlPath, original, "utf8");
  }
}

main().catch((e) => {
  console.error("[build-msi] Failed:", e?.message || e);
  process.exit(1);
});
