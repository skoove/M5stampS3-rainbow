### !! IMPORTANT STEPS TAKEN FOR THIS TO WORK !! ###
### !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!! ###A
# 1. nix-ld must be enabled
# 2. run espup
# 3. copy the script it generates in ~ to shellhook
# 4. you will need to make a symlink from a working version of rust analyser to the
#    esp tool chain for it to work right, see https://www.reddit.com/r/rust/comments/13d2tls/rustanalyzer_and_toolchain_for_esp/

{ pkgs ? import <nixpkgs> { } }:
let
  overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
  libPath = with pkgs; lib.makeLibraryPath [
    # load external libraries that you need in your rust project here
    stdenv.cc.cc.lib
    libxml2
    zlib
  ];
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    #clang
    # Replace llvmPackages with llvmPackages_X, where X is the latest LLVM version (at the time of writing, 16)
    #llvmPackages.bintools
    espup
    rustup
    cargo
    rustc
    probe-rs
    python3
    ldproxy
    flex 
    bison 
    gperf 
    python3
    ccache 
    glibc
    libffi 
    # rust-analyzer
    dfu-util
    libusb1
    dfu-util
    ninja
    cargo-generate
    libffi
    espflash
  ];
  RUSTC_VERSION = overrides.toolchain.channel;
  # https://github.com/rust-lang/rust-bindgen#environment-variables
  #LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
  shellHook = ''
    export LIBCLANG_PATH="/home/zie/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-19.1.2_20250225/esp-clang/lib"
    export PATH="/home/zie/.rustup/toolchains/esp/xtensa-esp-elf/esp-14.2.0_20240906/xtensa-esp-elf/bin:$PATH"

    fish ./secrets.fish
  '';
  # Add precompiled library to rustc search path
  RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
    # add libraries here (e.g. pkgs.libvmi)
  ]);
  LD_LIBRARY_PATH = libPath;
}
