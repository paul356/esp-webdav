mod esp_wifi;
mod dav_handler;
mod sd_card;

use esp_idf_sys as _;
use esp_idf_sys::{esp, esp_app_desc};
use esp_idf_svc::sys::{esp_vfs_fat_mount_config_t, CONFIG_WL_SECTOR_SIZE, esp_vfs_fat_spiflash_mount_rw_wl, wl_handle_t, esp_vfs_fat_info};
use std::ffi::CString;
use log::info;
use std::convert::Infallible;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

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

fn mount_builtin_fat(mount_point: &str) -> anyhow::Result<()> {
    info!("Mounting FAT filesystem...");
    let fat_config = esp_vfs_fat_mount_config_t {
        format_if_mount_failed: true,
        max_files: 4,
        allocation_unit_size: CONFIG_WL_SECTOR_SIZE as usize,
        disk_status_check_enable: false,
        use_one_fat: false,
    };

    let mut wl_handle: wl_handle_t = 0;
    let base_path = CString::new(mount_point)?;
    let partition_label = CString::new("storage")?;

    esp!( unsafe {
        esp_vfs_fat_spiflash_mount_rw_wl(
            base_path.as_ptr(),
            partition_label.as_ptr(),
            &fat_config,
            &mut wl_handle,
        )
    })?;

    Ok(())
}

async fn test_file_perf(mount_point: &str) -> anyhow::Result<()> {
    let file_path = format!("{}/habanera.mp4", mount_point);

    tokio::task::spawn(async move {
        let mut file = tokio::fs::File::open(file_path).await?;
        let mut read_buf = std::vec![0;32*1024];
        let mut read_size: usize = 0;

        let beg = tokio::time::Instant::now();

        for _i in 0..400 {
            let size = file.read(&mut read_buf).await?;
            read_size += size;
        }

        info!("[Async] Read {} bytes in {:?}", read_size, beg.elapsed());

        Ok::<_, anyhow::Error>(())
    }).await?
}

async fn test_wfile_perf(mount_point: &str) -> anyhow::Result<()> {
    let file_path = format!("{}/wfile_async.bin", mount_point);

    tokio::task::spawn(async move {
        let mut file = tokio::fs::File::create(file_path).await?;
        let mut write_buf = std::vec![0xfu8;32*1024];
        let mut write_size: usize = 0;

        let beg = tokio::time::Instant::now();

        for _i in 0..400 {
            let size = file.write(&mut write_buf).await?;
            write_size += size;
        }

        info!("[Async] Write {} bytes in {:?}", write_size, beg.elapsed());

        Ok::<_, anyhow::Error>(())
    }).await?
}

fn test_file_sync(mount_point: &str) -> anyhow::Result<()> {
    let file_path = format!("{}/habanera.mp4", mount_point);
    let mut file = std::fs::File::open(file_path)?;
    let mut read_buf = std::vec![0;32*1024];
    let mut read_size: usize = 0;

    let beg = tokio::time::Instant::now();

    use std::io::Read;
    for _i in 0..400 {
        let size = file.read(&mut read_buf).unwrap();
        read_size += size;
    }

    info!("[Sync] Read {} bytes in {:?}", read_size, beg.elapsed());

    Ok::<_, anyhow::Error>(())
}

fn test_wfile_sync(mount_point: &str) -> anyhow::Result<()> {
    let file_path = format!("{}/wfile_sync.bin", mount_point);
    let mut file = std::fs::File::create(file_path)?;
    let mut write_buf = std::vec![3u8;32*1024];
    let mut write_size: usize = 0;

    let beg = tokio::time::Instant::now();

    use std::io::Write;
    for _i in 0..400 {
        let size = file.write(&mut write_buf).unwrap();
        write_size += size;
    }

    info!("[Sync] Write {} bytes in {:?}", write_size, beg.elapsed());

    Ok::<_, anyhow::Error>(())
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    register_eventfd()?;

    const MOUNT_POINT: &str = "/vfat";

    let mut sd = sd_card::SdCard::new(MOUNT_POINT);
    sd.mount_spi()?;
    //mount_builtin_fat(MOUNT_POINT)?;

    let mpoint = CString::new(MOUNT_POINT).unwrap();
    let mut total_size = 0u64;
    let mut free_size  = 0u64;
    let _err = unsafe {
        esp_vfs_fat_info(
            mpoint.as_ptr(),
            &mut total_size as *mut u64,
            &mut free_size as *mut u64
        )
    };
    info!("Vfs Total size: {}, Free size: {}", total_size, free_size);

    //test_file_sync(MOUNT_POINT);
    test_wfile_sync(MOUNT_POINT);

    let mut wifi = esp_wifi::WifiLoop::new()?;

    info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .thread_stack_size(32 * 1024)
        .build()?
        .block_on(async move {
            wifi.configure().await?;
            wifi.initial_connect().await?;

            //test_file_perf(MOUNT_POINT).await?;
            test_wfile_perf(MOUNT_POINT).await?;

            tokio::spawn(dav_handler::hyper_server(MOUNT_POINT, 3000));
            
            info!("Entering main Wi-Fi run loop...");
            wifi.stay_connected().await?;

            Ok::<_, anyhow::Error>(())
        })?;

    Ok(())
}

