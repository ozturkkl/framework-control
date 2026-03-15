# Builds framework-control in two phases within a single derivation:
#   1. npm/Vite produces the static web UI (web/dist/)
#   2. cargo embeds that dist via rust-embed and produces the binary
{
  lib,
  rustPlatform,
  nodejs,
  fetchNpmDeps,
}:

let
  version = "0.5.2";

  # Pre-fetch npm dependencies for the offline build.
  # Update the hash by running: nix build 2>&1 | grep "got:"
  npmDeps = fetchNpmDeps {
    name = "framework-control-npm-deps";
    src = lib.cleanSourceWith {
      src = ../web;
      filter =
        name: _type:
        let
          base = baseNameOf (toString name);
        in
        base != "node_modules" && base != "dist" && base != ".vite";
    };
    hash = "sha256-j1xcy7ku3N+K54KEwRjcnH0qW22HaQEXq0t2U8EH99Y=";
  };

in

rustPlatform.buildRustPackage {
  pname = "framework-control";
  inherit version;

  src = lib.cleanSourceWith {
    src = ../.;
    filter =
      name: _type:
      let
        base = baseNameOf (toString name);
      in
      base != "target" && base != "node_modules" && base != "dist" && base != ".vite";
  };

  cargoLock = {
    lockFile = ../service/Cargo.lock;
  };

  # Cargo.lock lives in service/, not the source root.
  # cargoRoot tells cargoSetupPostPatchHook where to validate it;
  # buildAndTestSubdir tells cargoBuildHook to pushd there before cargo runs.
  cargoRoot = "service";
  buildAndTestSubdir = "service";

  nativeBuildInputs = [ nodejs ];

  preBuild = ''
    pushd web

    # Suppress postinstall so npm ci doesn't trigger gen:api
    # (which tries to compile cargo just to produce openapi.json).
    node -e "
      const fs = require('fs');
      const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
      delete pkg.scripts.postinstall;
      fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2));
    "

    # Install npm deps from the pre-fetched offline cache.
    npm_config_cache="${npmDeps}" npm ci --prefer-offline

    # Generate icons (src/api/ is committed so no codegen step needed).
    node scripts/gen-icons.mjs

    # Call vite via node explicitly — the node_modules/.bin/vite shebang uses
    # /usr/bin/env which doesn't exist in the Nix sandbox.
    node node_modules/vite/bin/vite.js build

    popd
  '';

  buildFeatures = [ "embed-ui" ];
  doCheck = false;

  # Remaining compile-time config baked via option_env!() in the service source.
  FRAMEWORK_CONTROL_PORT = "30912";
  FRAMEWORK_CONTROL_ALLOWED_ORIGINS = "http://127.0.0.1:5174,http://localhost:5174,https://ozturkkl.github.io";
  FRAMEWORK_CONTROL_UPDATE_REPO = "ozturkkl/framework-control";

  postInstall = ''
    mv $out/bin/framework-control-service $out/bin/framework-control
  '';

  meta = with lib; {
    description = "Lightweight control surface for Framework devices";
    homepage = "https://github.com/ozturkkl/framework-control";
    license = licenses.mit;
    platforms = [ "x86_64-linux" ];
    mainProgram = "framework-control";
  };
}
