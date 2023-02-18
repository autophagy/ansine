{ naersk-lib }:
naersk-lib.buildPackage {
  root = ./.;
  doCheck = true;
}
