# T2 Fan Control

<img src="assets/fancontrol.svg" alt="T2 Fan Control logo" width="96">

Fan controller application written in Rust for T2 Macs running Linux.


T2 Fan Control provides a compact desktop interface for monitoring temperatures, editing the fan curve and switching between presets.

Fan control is handled by a background daemon integrated with systemd. This keeps the state persistent across boot, suspend and resume, while the GUI talks to the daemon over a Unix socket.

## Installation

1. Download or clone the repository
2. Unpack it
3. Run:

```bash
make
sudo make install
```
This step is mandatory. It puts the binary, desktop entry, icon and systemd service in the correct system locations, then enables and starts `t2-fancontrol.service`.

If you are using t2fanrd (check that by running `sudo systemctl status t2fanrd`) we need to disable it:

```bash
sudo systemctl disable t2fanrd
``` 


## Build Dependencies

You need a Rust toolchain and the usual native build dependencies for GTK4 and Libadwaita.

At minimum:

- `cargo`
- `make`
- `pkg-config`
- `glib-compile-resources`
- GTK4 development files
- Libadwaita development files

For example on Fedora this typically means packages such as:

- `gtk4-devel`
- `libadwaita-devel`
- `pkgconf-pkg-config`

## Support

[Fund my bugs](https://donate.stripe.com/eVq14n8a7agh2lQdqq14400)
