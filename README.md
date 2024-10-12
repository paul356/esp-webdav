## What is this repository for?
This repository is a Rust on ESP project that demonstrates how to use run a webdav server on a esp32 board. I am surprised by the power of Rust. By using Rust crates and the esp-idf library I can easily build a webdav server for esp32 boards.

NOTE 2024/10/11: I face a critical problem with using dav-server. The stack size is less than 20 kb using hyper only. But the stack size overshots when I include dav-server. It constantly causes stackoverflow issues on esp32s3. It is probably due to extensive usage of futures in the dav-server crate. I will try to investigate it further. If I can't workarround it, I will try to implement a simple webdav server myself.
UPDATE 2024/10/12: Good news. I don't how to implement a webdav server myself. It turns out that I need to specify a stack size for my created tokio executer. The default stack size of task thread is way too small as of 1KB. I increase it to 32KB and the stackoverflow issue is gone. Now I can run dav-server on my esp32s3 dev-kit. Though there is still some issues but I think they are solvable.

## Pre-requisites
- Install Rust
  - Follow [rustup](https://rustup.rs/) to install Rust.
- Install espup
  - Follow [Setting Up a Development Environment](https://docs.esp-rs.org/book/installation/index.html) chapter of The Rust on ESP Book to set up a RUST on ESP development environment.
- Install esp-idf
  - Install esp-idf following this [guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/linux-macos-setup.html#step-1-install-prerequisites). It is recommended to install pre-requisites in The Rust on ESP Book. In this project I use an existing esp-idf installation to speed up the build process. And I have issues with esp-idf 5.2 when mounting a vfat partition. So I use esp-idf 5.3 which is the lastest stable version.

## Build and Flash
- Clone the git repository

        git clone $HOME/code/esp/esp-idf/export.sh

- Build image

        . $HOME/export-esp.sh
        . $HOME/code/esp/esp-idf/export.sh
        env WIIF_SSID=YOUR_WIFI WIFI_PASS=YOUR_WIFI_PASSWORD cargo build

- Flash the image to the board
    
        env WIFI_SSID=YOUR_WIFI WIFI_PASS=YOUR_WIFI_PASSWORD cargo espflash flash

- Check the output

        cargo espflash monitor

## References
- [esp32-tokio-demo](https://github.com/jasta/esp32-tokio-demo/tree/main) by jasta. Great work.
- [esp-idf-template](https://github.com/esp-rs/esp-idf-template) for how to generate a Rust on ESP template project.
- [esp-idf-sys document](https://docs.esp-rs.org/esp-idf-sys/esp_idf_sys/) for how to customize a Rust on ESP project.
