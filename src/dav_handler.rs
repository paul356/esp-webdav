use dav_server::{fakels::FakeLs, localfs::LocalFs, DavHandler};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::info;
use std::convert::Infallible;
use tokio::net::TcpListener;
use esp_idf_sys::{heap_caps_get_free_size, heap_caps_get_largest_free_block, MALLOC_CAP_8BIT};

pub async fn hyper_server(srv_root: &str, port: u32) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);

    tokio::task::spawn_blocking(|| {
        unsafe { esp_idf_sys::sleep(1); }
    });

    let dav_server = Box::new(DavHandler::builder()
                              .filesystem(LocalFs::new(srv_root, false, false, false))
                              .locksystem(FakeLs::new())
                              .build_handler());

    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let dav_server = dav_server.clone();

        // Use an adapter to access something implementing `tokio::io` traits
        // as if they implement `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {

            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new().serve_connection(
                io,
                service_fn(move |req| {
                    let dav_server = dav_server.clone();
                    async move {
                        let free_size = unsafe { heap_caps_get_free_size(MALLOC_CAP_8BIT) };
                        let max_size = unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_8BIT) };
                        info!("Mem Free size: {}, Max size: {}", free_size, max_size);

                        info!("accept webdav request {}", req.uri());
                        let uri = req.uri().clone();

                        let resp = dav_server.handle(req).await;

                        info!("done with webdav request {}", uri);
                        Ok::<_, Infallible>(resp)
                    }
                })).await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
