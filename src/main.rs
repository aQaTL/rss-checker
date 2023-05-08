extern crate core;

use std::future::ready;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::make_service_fn;
use hyper::Server;
use log::info;
use onlyerror::Error;
use tokio::runtime::Runtime;

use crate::rss::Entries;

mod rss;
mod rss_service;
mod templater;

#[derive(Debug)]
struct Args {
	host: String,
	port: u16,
}

impl Args {
	fn new() -> Result<Self, pico_args::Error> {
		let mut args = pico_args::Arguments::from_env();
		let host = args
			.opt_value_from_str("--host")?
			.unwrap_or_else(|| String::from("0.0.0.0"));
		let port = args.opt_value_from_str("--port")?.unwrap_or(8080);

		Ok(Args { host, port })
	}
}

fn main() {
	aqa_logger::init();

	let args = match Args::new() {
		Ok(v) => v,
		Err(err) => {
			eprintln!("Error: {err}");
			return;
		}
	};

	info!("{args:#?}");

	if let Err(err) = run(args) {
		eprintln!("{err}");
		std::process::exit(127);
	}
}

#[derive(Debug, Error)]
enum Error {
	#[error("invalid address or port")]
	InvalidAddress(#[from] std::net::AddrParseError),

	#[error("http server: {0}")]
	Hyper(#[from] hyper::Error),
}

fn run(args: Args) -> Result<(), Error> {
	let tokio_runtime = Runtime::new().expect("Failed to build tokio runtime");
	let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

	let guard = tokio_runtime.enter();

	info!("Bind address: {addr}");
	let server = Server::bind(&addr);

	drop(guard);

	let entries = Entries::new();

	tokio_runtime.block_on(server.serve(make_service_fn(|_addr_stream| {
		let entries = Arc::clone(&entries);
		ready(Result::<rss_service::Service, rss_service::Error>::Ok(
			rss_service::Service::new(entries),
		))
	})))?;

	Ok(())
}
