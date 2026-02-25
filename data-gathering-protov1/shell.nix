{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    pkgs.openssl
  ];

  buildInputs = with pkgs; [
    systemd
  ];
}

