{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11"; };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      makeTest = pkgs.callPackage "${nixpkgs}/nixos/tests/make-test-python.nix";
    in
    {
      checks.${system}.test-diesel-migration =
        let
          username = "postgres";
          password = "password";
          database = "database";
          migrations_dir = "migrations-based";
        in
        makeTest
          {
            name = "test-diesel-migration";
            nodes = {
              server = { lib, config, pkgs, ... }: {
                services.postgresql = {
                  enable = true;
                  ensureDatabases = [ database ];
                  ensureUsers = [{
                    name = username;
                    ensurePermissions = {
                      "DATABASE ${database}" = "ALL PRIVILEGES";
                    };
                  }];
                  initialScript = pkgs.writeScript "initScript" ''
                    ALTER USER postgres WITH PASSWORD '${password}';
                  '';
                };

                systemd.services.postgresql.postStart = lib.mkAfter ''
                  ${pkgs.diesel-cli}/bin/diesel migration run --database-url "postgres://${username}:${password}@localhost/${database}" --migration-dir ${self}/${migrations_dir}
                  ${pkgs.diesel-cli}/bin/diesel migration redo --database-url "postgres://${username}:${password}@localhost/${database}" --migration-dir ${self}/${migrations_dir}
                  ${pkgs.diesel-cli}/bin/diesel migration run --database-url "postgres://${username}:${password}@localhost/${database}" --migration-dir ${self}/${migrations_dir}
                '';
              };
            };
            testScript = ''
              start_all()
              server.wait_for_unit("postgresql.service")
              server.succeed("sudo -u postgres -- ${pkgs.diesel-cli}/bin/diesel print-schema --database-url postgres://${username}:${password}@localhost/${database} > schema.rs")
              server.copy_from_vm("schema.rs", "")
            '';
          }
          {
            inherit pkgs;
            inherit (pkgs) system;
          };

      packages.${system} = {
        update-schema = pkgs.writeScriptBin "update-schema" ''
          nix build ${self}#checks.${system}.test-diesel-migration
          BUILD_DIR=$(nix build ${self}#checks.${system}.test-diesel-migration --no-link --print-out-paths)
          rm -rf src/schema.rs
          cp $BUILD_DIR/schema.rs src/schema.rs
        '';

        run-migration-based = pkgs.writeScriptBin "run-migration" ''
          ${pkgs.diesel-cli}/bin/diesel migration run --migration-dir ${self}/migrations-based
        '';
      };

      devShells."x86_64-linux".default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [ pkg-config protobuf ];
        buildInputs=  with pkgs; [ openssl ];
      };
    };
}
