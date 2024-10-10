use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{esp_vfs_fat_mount_config_t, CONFIG_WL_SECTOR_SIZE, esp_vfs_fat_spiflash_mount_rw_wl, wl_handle_t};
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_sys as _;
use esp_idf_sys::{esp, esp_app_desc, EspError};
use log::info;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::fs::File;
use std::ffi::CString;
use std::num::NonZero;
use std::convert::Infallible;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;

// Edit these or provide your own way of provisioning...
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");

// This is a macro provided by the build script that generates a static reference to the
esp_app_desc!();

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // eventfd is needed by our mio poll implementation.  Note you should set max_fds
    // higher if you have other code that may need eventfd.
    info!("Setting up eventfd...");
    let config = esp_idf_sys::esp_vfs_eventfd_config_t {
        max_fds: 1,
        ..Default::default()
    };
    esp! { unsafe { esp_idf_sys::esp_vfs_eventfd_register(&config) } }?;

    info!("Setting up board...");
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let timer = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Mounting FAT filesystem...");
    let fat_config = esp_vfs_fat_mount_config_t {
        format_if_mount_failed: true,
        max_files: 4,
        allocation_unit_size: CONFIG_WL_SECTOR_SIZE as usize,
        disk_status_check_enable: false,
        use_one_fat: false,
    };

    let mut wl_handle: wl_handle_t = 0;
    let base_path = CString::new("/vfat")?;
    let partition_label = CString::new("storage")?;

    let _ = unsafe {
        esp_vfs_fat_spiflash_mount_rw_wl(
            base_path.as_ptr(),
            partition_label.as_ptr(),
            &fat_config,
            &mut wl_handle,
        )
    };

    /*
    let mut file = File::create("/vfat/hello.txt")?;
    file.write_all(b"Hello, world!")?;
    drop(file);
    */

    info!("Initializing Wi-Fi...");
    let wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
        timer.clone(),
    )?;

    info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let mut f = File::open("/vfat/hello.txt").await.map_err(|_| EspError::from_non_zero(NonZero::new(1).unwrap()))?;
            let mut buffer = [0; 20];

            // read up to 10 bytes
            let n = f.read(&mut buffer).await.map_err(|_| EspError::from_non_zero(NonZero::new(1).unwrap()))?;
            info!("content of hello.txt: {:?}", String::from_utf8(buffer[..n].to_vec()));
            
            let mut wifi_loop = WifiLoop { wifi };
            wifi_loop.configure().await?;
            wifi_loop.initial_connect().await?;

            tokio::spawn(hyper_server());
            
            info!("Entering main Wi-Fi run loop...");
            wifi_loop.stay_connected().await
        })?;

    Ok(())
}

async fn hyper_server() -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:3000");

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(&addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
            // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(hello))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

pub struct WifiLoop<'a> {
    wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> WifiLoop<'a> {
    pub async fn configure(&mut self) -> Result<(), EspError> {
        info!("Setting Wi-Fi credentials...");
        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: WIFI_SSID.try_into().unwrap(),
                password: WIFI_PASS.try_into().unwrap(),
                ..Default::default()
            }))?;

        info!("Starting Wi-Fi driver...");
        self.wifi.start().await
    }

    pub async fn initial_connect(&mut self) -> Result<(), EspError> {
        self.do_connect_loop(true).await
    }

    pub async fn stay_connected(mut self) -> Result<(), EspError> {
        self.do_connect_loop(false).await
    }

    async fn do_connect_loop(&mut self, exit_after_first_connect: bool) -> Result<(), EspError> {
        let wifi = &mut self.wifi;
        loop {
            // Wait for disconnect before trying to connect again.  This loop ensures
            // we stay connected and is commonly missing from trivial examples as it's
            // way too difficult to showcase the core logic of an example and have
            // a proper Wi-Fi event loop without a robust async runtime.  Fortunately, we can do it
            // now!
            wifi.wifi_wait(|wifi| wifi.is_up(), None).await?;

            info!("Connecting to Wi-Fi...");
            wifi.connect().await?;

            info!("Waiting for association...");
            wifi.ip_wait_while(|wifi| wifi.is_up().map(|s| !s), None)
                .await?;

            if exit_after_first_connect {
                return Ok(());
            }
        }
    }
}
