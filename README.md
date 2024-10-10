## Pre-requisites
- Install Rust
  - Follow [rustup](https://rustup.rs/) to install Rust.
- Install espup
  - Follow [Setting Up a Development Environment](https://docs.esp-rs.org/book/installation/index.html) chapter of The Rust on ESP Book to set up a RUST on ESP development environment.
- Install esp-idf
  - It is recommended to install pre-requisites in The Rust on ESP Book. In this project I use an existing esp-idf installation to speed up the build process. And I have issues with esp-idf 5.2 when mounting a vfat partition. So I use esp-idf 5.3 which is the lastest stable version.

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
