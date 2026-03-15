{
  description = "Framework Control - lightweight control surface for Framework devices";

  inputs.nixpkgs.url = "github:ozturkkl/nixpkgs/framework-control";

  outputs =
    { nixpkgs, ... }:
    {
      packages.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.framework-control;
      nixosModules.default = import "${nixpkgs}/nixos/modules/services/hardware/framework-control.nix";
    };
}
