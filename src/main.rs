mod esp_wifi;
mod dav_handler;
mod sd_card;

use esp_idf_sys as _;
use esp_idf_sys::{esp, esp_app_desc};
use log::info;

// This is a macro provided by the build script that generates a static reference to the
esp_app_desc!();

fn register_eventfd() -> anyhow::Result<()> {
    // eventfd is needed by our mio poll implementation.  Note you should set max_fds
    // higher if you have other code that may need eventfd.
    info!("Setting up eventfd...");
    let config = esp_idf_sys::esp_vfs_eventfd_config_t {
        max_fds: 1,
        ..Default::default()
    };
    esp! { unsafe { esp_idf_sys::esp_vfs_eventfd_register(&config) } }?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    register_eventfd()?;

    const MOUNT_POINT: &str = "/vfat";

    let mut sd = sd_card::SdCard::new(MOUNT_POINT);
    sd.mount()?;

    let mut wifi = esp_wifi::WifiLoop::new()?;

    info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_stack_size(64 * 1024)
        .build()?
        .block_on(async move {
            wifi.configure().await?;
            wifi.initial_connect().await?;

            tokio::spawn(dav_handler::hyper_server(MOUNT_POINT, 3000));
            
            info!("Entering main Wi-Fi run loop...");
            wifi.stay_connected().await
        })?;

    Ok(())
}

