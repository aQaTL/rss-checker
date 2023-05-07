use std::future::{ready, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_core::Stream;
use hyper::body::{Bytes, HttpBody};
use hyper::{Body, Method, Request, Response, StatusCode};
use log::{debug, info};
use nom::bytes::complete::{tag, take_till, take_while};
use nom::sequence::{preceded, separated_pair};
use onlyerror::Error;
use tokio::sync::RwLock;

use crate::Entries;

#[derive(Debug, Error)]
pub enum Error {
	#[error("malformed add_entry form payload")]
	ParseAddEntryForm(nom::Err<nom::error::Error<String>>),
}

pub struct Service {
	visits: u64,
	entries: Arc<RwLock<Entries>>,
}

impl Service {
	pub fn new(entries: Arc<RwLock<Entries>>) -> Self {
		Service { visits: 0, entries }
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
				Box::pin(handle_index(req, Arc::clone(&self.entries), self.visits))
			}

			(Method::POST, ["add_entry"]) => {
				Box::pin(handle_add_entry(req, Arc::clone(&self.entries)))
			}

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

async fn handle_index(
	_req: Request<Body>,
	entries: Arc<RwLock<Entries>>,
	_visits: u64,
) -> Result<Response<Body>, Error> {
	let index_file = std::fs::read_to_string("./website/index.html").unwrap();
	// let index_file = index_file.replace("{visitors}", &visits.to_string());

	let entries_g = entries.read().await;
	debug!("Entries: {:#?}", entries_g.entries);
	drop(entries_g);

	Ok(Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", "text/html")
		.body(Body::wrap_stream(StreamingIndex {
			entries,
			index_file,
			sent: 0,
		}))
		.unwrap())
}

struct StreamingIndex {
	#[allow(dead_code)]
	entries: Arc<RwLock<Entries>>,

	index_file: String,
	sent: usize,
}

impl Stream for StreamingIndex {
	type Item = Result<Bytes, std::io::Error>;

	fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.sent == self.index_file.len() {
			return Poll::Ready(None);
		}

		let bytes = Bytes::from(std::mem::take(&mut self.index_file));
		self.sent = bytes.len();
		Poll::Ready(Some(Ok(bytes)))
	}
}

async fn handle_add_entry(
	req: Request<Body>,
	entries: Arc<RwLock<Entries>>,
) -> Result<Response<Body>, Error> {
	let mut body = req.into_body();

	let mut bytes = Vec::new();

	while let Some(chunk) = body.data().await {
		let chunk = chunk.unwrap();
		bytes.extend_from_slice(&chunk);
	}

	let body = String::from_utf8(bytes).unwrap();
	info!("Body: {body}");

	// TODO: `?` here doesn't work nicely. We need a wrapper function like handle_response .
	let new_entry: AddEntryForm =
		dbg!(parse_add_entry_form(&body).map_err(Error::ParseAddEntryForm))?;

	let mut entries_g = entries.write().await;
	entries_g.entries.insert(new_entry.name, new_entry.url);
	drop(entries_g);

	Ok(Response::builder()
		.status(StatusCode::SEE_OTHER)
		.header("Content-Type", "text/html")
		.header("Location", "/")
		.body("<html></html>".into())
		.unwrap())
}

#[derive(Debug)]
struct AddEntryForm {
	name: String,
	url: String,
}

fn parse_add_entry_form(input: &str) -> Result<AddEntryForm, nom::Err<nom::error::Error<String>>> {
	separated_pair(
		preceded(tag("entry_name="), take_till(|c: char| c == '&')),
		tag("&"),
		preceded(tag("entry_url="), take_while(|c: char| c.is_ascii())),
	)(input)
	.map(|(_input, (name, url))| AddEntryForm {
		//TODO these are x-www-form-urlencoded, we need to decode that
		name: name.to_string(),
		url: url.to_string(),
	})
	.map_err(|err: nom::Err<nom::error::Error<&str>>| err.to_owned())
}
