{
  description = "Framework Control - lightweight control surface for Framework devices";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    {
      packages.x86_64-linux.default =
        nixpkgs.legacyPackages.x86_64-linux.callPackage ./nix/package.nix
          { };

      nixosModules.default = import ./nix/module.nix;
    };
}
