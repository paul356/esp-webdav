use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_sys::EspError;
use log::info;
use tokio::time::sleep;

// Edit these or provide your own way of provisioning...
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");

pub struct WifiLoop<'a> {
    wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> WifiLoop<'a> {
    pub fn new() -> Result<Self, EspError> {
        let peripherals = Peripherals::take().unwrap();
        let sysloop = EspSystemEventLoop::take()?;
        let timer = EspTaskTimerService::new()?;
        let nvs = EspDefaultNvsPartition::take()?;

        let wifi = AsyncWifi::wrap(
            EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
            sysloop,
            timer,
        )?;

        Ok(Self { wifi })
    }
    
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
        let mut err_cnt = 0;
        loop {
            // Wait for disconnect before trying to connect again.  This loop ensures
            // we stay connected and is commonly missing from trivial examples as it's
            // way too difficult to showcase the core logic of an example and have
            // a proper Wi-Fi event loop without a robust async runtime.  Fortunately, we can do it
            // now!
            wifi.wifi_wait(|wifi| wifi.is_up(), None).await?;

            info!("Connecting to Wi-Fi...");
            let conn_res = wifi.connect().await;
            if let Err(esp_err) = conn_res {
                info!("Failed to connect to Wi-Fi: {:?}", esp_err);
                sleep(std::time::Duration::from_secs(1)).await;
                err_cnt += 1;
            } else {
                info!("Waiting for association...");
                wifi.ip_wait_while(|wifi| wifi.is_up().map(|s| !s), None)
                    .await?;
                err_cnt = 0;
            }

            if exit_after_first_connect && (err_cnt == 0 || err_cnt >= 5) {
                return Ok(());
            }
        }
    }
}

