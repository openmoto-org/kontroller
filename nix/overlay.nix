final: prev: {
  rust-esp = prev.callPackage ./rust-esp.nix { };
  rust-src-esp = prev.callPackage ./rust-src-esp.nix { };

  esp-idf-esp32s3-with-clang = final.esp-idf-full.override
    {
      toolsToInclude = [
        "esp-clang"
        "xtensa-esp32s3-elf"
        "esp32ulp-elf"
        "openocd-esp32"
        "xtensa-esp-elf-gdb"
      ];
    };
}
