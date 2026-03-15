{
  description = "Framework Control - lightweight control surface for Framework devices";

  inputs.nixpkgs.url = "github:ozturkkl/nixpkgs/framework-control";

  outputs =
    { self, nixpkgs, ... }:
    {
      packages.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.framework-control;

      nixosModules.default =
        { lib, pkgs, ... }:
        {
          imports = [ "${nixpkgs}/nixos/modules/services/hardware/framework-control.nix" ];
          # Supply the package from the fork since it isn't in the user's nixpkgs yet
          services.framework-control.package = lib.mkDefault self.packages.${pkgs.system}.default;
        };
    };
}
