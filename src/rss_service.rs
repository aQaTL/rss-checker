use std::future::{ready, Future};
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::body::HttpBody;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::{debug, info};
use onlyerror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("TODO: REMOVE ME")]
	Test,
}

pub struct Service {
	visits: u64,
}

impl Service {
	pub fn new() -> Self {
		Service { visits: 0 }
	}
}

impl hyper::service::Service<Request<Body>> for Service {
	type Response = Response<Body>;
	type Error = Error;
	#[allow(clippy::type_complexity)]
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		debug!("req: {req:?}");

		let uri_path = req.uri().path().to_owned();
		let path: Vec<&str> = split_uri_path(&uri_path).collect();
		let method = req.method().clone();
		debug!("path: {:?}", path.as_slice());
		self.visits += 1;

		match (method, path.as_slice()) {
			(Method::GET, []) => {
				let index_file = std::fs::read_to_string("./website/index.html").unwrap();
				let index_file = index_file.replace("{visitors}", &self.visits.to_string());
				Box::pin(ready(Ok(Response::builder()
					.status(StatusCode::OK)
					.header("Content-Type", "text/html")
					.body(index_file.into())
					.unwrap())))
			}

			(Method::POST, ["add_entry"]) => Box::pin(handle_add_entry(req)),

			_ => Box::pin(ready(Ok(Response::builder()
				.status(StatusCode::OK)
				.body(format!("Hello world! We've had {} visits already!", self.visits).into())
				.unwrap()))),
		}
	}
}

pub fn split_uri_path(path: &str) -> impl Iterator<Item = &str> {
	path.split('/').filter(|segment| !segment.is_empty())
}

async fn handle_add_entry(req: Request<Body>) -> Result<Response<Body>, Error> {
	let mut body = req.into_body();

	let mut bytes = Vec::new();

	while let Some(chunk) = body.data().await {
		let chunk = chunk.unwrap();
		bytes.extend_from_slice(&chunk);
	}

	let body = String::from_utf8(bytes).unwrap();
	info!("Body: {body}");

	Ok(Response::builder()
		.status(StatusCode::SEE_OTHER)
		.header("Content-Type", "text/html")
		.header("Location", "/")
		.body("<html></html>".into())
		.unwrap())
}
