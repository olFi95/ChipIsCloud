#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
extern crate alloc;

use defmt::*;
use emballoc::Allocator;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Ipv4Cidr, StackResources};
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::eth::{Ethernet, PacketQueue};
use embassy_stm32::peripherals::ETH;
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllDiv, PllMul, PllPreDiv, PllSource, Sysclk,
    VoltageScale,
};
use embassy_stm32::rng::Rng;
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, eth, peripherals, rng, Config};
use embassy_time::Duration;
use heapless::Vec;
use log::{error, info};
use picoserve::routing::get_service;
use picoserve::{make_static, AppBuilder, AppRouter};
use rand_core::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const WEB_TASK_POOL_SIZE: usize = 1;

#[global_allocator]
static ALLOCATOR: Allocator<4096> = Allocator::new();

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

type Device = Ethernet<'static, ETH, GenericSMI>;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
    runner.run().await
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    config.rcc.hsi = None;
    config.rcc.hsi48 = Some(Default::default()); // needed for RNG
    config.rcc.hse = Some(Hse {
        freq: Hertz(8_000_000),
        mode: HseMode::BypassDigital,
    });
    config.rcc.pll1 = Some(Pll {
        source: PllSource::HSE,
        prediv: PllPreDiv::DIV2,
        mul: PllMul::MUL125,
        divp: Some(PllDiv::DIV2),
        divq: Some(PllDiv::DIV2),
        divr: None,
    });
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV1;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    config.rcc.apb3_pre = APBPrescaler::DIV1;
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.voltage_scale = VoltageScale::Scale0;
    let p = embassy_stm32::init(config);
    info!("Hello World!");

    // Generate random seed.
    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    rng.fill_bytes(&mut seed);
    let seed = u64::from_le_bytes(seed);

    let mac_addr = [0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];

    static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
    let device = Ethernet::new(
        PACKETS.init(PacketQueue::<4, 4>::new()),
        p.ETH,
        Irqs,
        p.PA1,
        p.PA2,
        p.PC1,
        p.PA7,
        p.PC4,
        p.PC5,
        p.PG13,
        p.PB15,
        p.PG11,
        GenericSMI::new(0),
        mac_addr,
    );

    // let config = embassy_net::Config::dhcpv4(Default::default());
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 177, 222), 24),
        dns_servers: Vec::new(),
        gateway: Some(Ipv4Address::new(192, 168, 177, 1)),
    });

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) =
        embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);

    // Launch network task
    spawner.must_spawn(net_task(runner));

    // Ensure DHCP configuration is up before trying connect
    stack.wait_config_up().await;

    println!("Network task initialized");

    let app = make_static!(AppRouter<AppProps>, AppProps.build_app());

    let config = make_static!(
        picoserve::Config<Duration>,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    for id in 0..WEB_TASK_POOL_SIZE {
        println!("spawned web task {}", id);
        spawner.must_spawn(web_task(id, stack, app, config));
    }
}
#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &cortex_m_rt::ExceptionFrame) -> ! {
    error!("HardFault at {:#?}", ef);
    loop {}
}

struct AppProps;

impl AppBuilder for AppProps {
    // type PathRouter = ();
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        picoserve::Router::new()
            .route(
                "/",
                get_service(picoserve::response::File::html(include_str!(
                    "../../target/www/index.html"
                ))),
            )
            .route(
                "/web.js",
                get_service(picoserve::response::File::with_content_type(
                    "text/javascript",
                    include_bytes!("../../target/www/web.js"),
                )),
            )
            .route(
                "/web_bg.wasm",
                get_service(picoserve::response::File::with_content_type(
                    "application/wasm",
                    include_bytes!("../../target/www/web_bg.wasm"),
                )),
            )
    }
}
#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<AppProps>,
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
