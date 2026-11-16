## Building from source

It is easy to build Antares IDE from source on Linux, Windows, or macOS. Cargo handles the build process; all you need to do is ensure the correct dependencies are installed.

1. Install the Rust compiler and Cargo using [`rustup.rs`](https://rustup.rs/). Ensure you are using the pinned Rust toolchain (1.98.0+).

2. Install dependencies for your operating system:

#### Ubuntu / Debian
```sh
sudo apt install clang libxkbcommon-x11-dev pkg-config libvulkan-dev libwayland-dev xorg-dev libxcb-shape0-dev libxcb-xfixes0-dev
```
#### Fedora
```sh
sudo dnf install clang libxkbcommon-x11-devel libxcb-devel vulkan-loader-devel wayland-devel openssl-devel pkgconf
```
#### Void Linux
```sh
sudo xbps-install -S base-devel clang libxkbcommon-devel vulkan-loader wayland-devel
```

3. Clone this repository:
```sh
git clone https://github.com/dhapidpad/AntaresIDE.git ~/AntaresIDE
```

4. `cd` into the repository, and run the build command:
```sh
cd ~/AntaresIDE
cargo install --path . --bin lapce --profile release-lto --locked
```

> If you use a different distribution and are having trouble finding appropriate dependencies, let us know in an issue!

Once compiled, the executable will be installed in `$HOME/.cargo/bin/lapce` and should be available in `PATH` automatically.
