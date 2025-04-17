use defmt::println;
use embassy_time::Duration;
use picoserve::{make_static, AppBuilder, AppRouter};
use picoserve::routing::get_service;

pub(crate) struct StaticSiteProps;

impl AppBuilder for StaticSiteProps {
    // type PathRouter = ();
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        picoserve::Router::new()
            .route(
                "/",
                get_service(picoserve::response::File::with_content_type_and_headers(
                    "text/html",
                    include_bytes!("../../target/www/index.html.gz"),
                    &[("Content-Encoding", "gzip")],
                )),
            )
            .route(
                "/web.js",
                get_service(picoserve::response::File::with_content_type_and_headers(
                    "text/javascript",
                    include_bytes!("../../target/www/web.js.gz"),
                    &[("Content-Encoding", "gzip")],
                )),
            )
            .route(
                "/web_bg.wasm",
                get_service(picoserve::response::File::with_content_type_and_headers(
                    "application/wasm",
                    include_bytes!("../../target/www/web_bg.wasm.gz"),
                    &[("Content-Encoding", "gzip")],
                )),
            )
    }
}

impl StaticSiteProps {
    pub fn get_static(self) -> &'static mut AppRouter<StaticSiteProps> {
        let app = make_static!(AppRouter<StaticSiteProps>, StaticSiteProps.build_app());
        app
    }
}
#[embassy_executor::task(pool_size = 1)]
pub async fn static_site_task(
    id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<StaticSiteProps>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];
    println!("web_task {} started", id);
    picoserve::listen_and_serve(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    )
    .await
}
