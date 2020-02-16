mod address;
mod consts;
mod http_client;
mod http_local;
mod socks5;
use log::info;
mod monitor;
use clap::{App, Arg};
use futures::{
    future::{self, Either},
    FutureExt,
};
use std::net::SocketAddr;
use tokio::runtime::Builder;
fn main() {
    let matches = App::new("socks2http")
        .version("0.1.0")
        .arg(
            Arg::with_name("LOCAL_ADDR")
                .short("l")
                .long("local-addr")
                .takes_value(true)
                .help("Local HTTP bind addr: `127.0.0.1:1081`"),
        )
        .arg(
            Arg::with_name("SOCKS5_ADDR")
                .short("s")
                .long("socks5-addr")
                .takes_value(true)
                .help("Socks5 proxy addr: `1.1.1.1:1080`"),
        )
        .about("A simple http proxy which converts socks5 to http")
        .get_matches();
    env_logger::init();

    let local_listen_addr: SocketAddr = match matches.value_of("LOCAL_ADDR") {
        Some(addr) => {
            let local_addr = addr.parse().unwrap();
            info!("Listen at: {}", local_addr);
            local_addr
        }
        _ => panic!("`local-addr`, should be provided"),
    };
    let proxy_address: SocketAddr = match matches.value_of("SOCKS5_ADDR") {
        Some(addr) => {
            let proxy_addr = addr.parse().unwrap();
            info!("Proxy Server: {}", proxy_addr);
            proxy_addr
        }
        _ => panic!("`socks5-addr`, should be provided"),
    };

    let mut builder = Builder::new();
    if cfg!(feature = "single-threaded") {
        builder.basic_scheduler();
    } else {
        builder.threaded_scheduler();
    }
    info!("This is socks2http by xVanTuring");
    let mut runtime = builder
        .enable_all()
        .build()
        .expect("Unable to create Tokio Runtime");
    runtime.block_on(async move {
        let abort_signal = monitor::create_signal_monitor();
        match future::select(
            http_local::run(local_listen_addr, proxy_address).boxed(),
            abort_signal.boxed(),
        )
        .await
        {
            // Server future resolved without an error. This should never happen.
            Either::Left((Ok(..), ..)) => panic!("Server exited unexpectly"),
            // Server future resolved with error, which are listener errors in most cases
            Either::Left((Err(err), ..)) => panic!("Server exited unexpectly with {}", err),
            // The abort signal future resolved. Means we should just exit.
            Either::Right(_) => (),
        }
    })
}
