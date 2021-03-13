How this directory is organized:

This is a Rust crate which contains the source code that defines the InitRD format, the code to
generate an InitRD from a directory structure, and the code that the kernel will use to read it.

The contents of the InitRD vary by target architecture. They're located in contents/. Each
subdirectory gets more specific about the exact architecture. Additionally, each can contain an
initrd/ directory.  The final InitRD is made by concatenating the contents of each initrd/ directory
for matching architectures, with more specific ones overriding more general ones. For example, this
structure

  + contents/
  +-+ initrd/
  | + cross-platform.txt (contents: "Basic functionality for all architectures")
  |
  +-+ aarch64/
    +-+ initrd/
    | + hello.txt (contents: "Hello from Aarch64")
    |
    +-+ virt/
    | +-+ initrd/
    |   + virt-driver.bin
    |
    +-+ raspi3/
      +-+ initrd/
        + hello.txt (contents: "Hello from Raspberry Pi 3")
        + raspi3-driver.bin

will yield this structure when building for aarch64-virt

  + initrd/
  + cross-platform.txt (contents: "Basic functionality for all architectures")
  + hello.txt (contents: "Hello from Aarch64")
  + virt-driver.bin

this structure when building for aarch64-raspi3

  + initrd/
  + cross-platform.txt (contents: "Basic functionality for all architectures")
  + hello.txt (contents: "Hello from Raspberry Pi 3")
  + raspi3-driver.bin

and this structure when building for anything else with aarch64 (e.g. aarch64-raspi4)

  + initrd/
  + cross-platform.txt (contents: "Basic functionality for all architectures")
  + hello.txt (contents: "Hello from Aarch64")
