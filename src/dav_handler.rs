use dav_server::{fakels::FakeLs, localfs::LocalFs, DavHandler};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::info;
use std::convert::Infallible;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use std::sync::Arc;

pub async fn hyper_server(srv_root: &str, port: u32) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);

    let dav_server = Box::new(DavHandler::builder()
                              .filesystem(LocalFs::new(srv_root, false, false, false))
                              .locksystem(FakeLs::new())
                              .build_handler());

    let listener = TcpListener::bind(addr).await?;

    let semaphore = Arc::new(Semaphore::new(2));

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let dav_server = dav_server.clone();

        // Use an adapter to access something implementing `tokio::io` traits
        // as if they implement `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        let thread_semaphore = semaphore.clone();
        tokio::task::spawn(async move {
            let thread_semaphore = thread_semaphore.clone();

            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new().serve_connection(
                io,
                service_fn(move |req| {
                    let thread_semaphore = thread_semaphore.clone();

                    let dav_server = dav_server.clone();
                    async move {
                        let permit = thread_semaphore.acquire().await.unwrap();

                        info!("accept webdav request {}", req.uri());
                        let uri = req.uri().clone();

                        let resp = dav_server.handle(req).await;

                        info!("done with webdav request {}", uri);
                        drop(permit);
                        Ok::<_, Infallible>(resp)
                    }
                })).await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
