use std::{
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};

use axum::{extract::connect_info::Connected, Router};
use bindable::BindableAddr;
use futures::ready;
use hyper::server::accept::Accept;
use tokio::net::{
	unix::{SocketAddr, UCred},
	UnixListener, UnixStream,
};

use super::Error;

struct UdsAccept(UnixListener);

impl UdsAccept {
	#[inline]
	fn new(path: &std::path::Path) -> std::io::Result<Self> {
		UnixListener::bind(path).map(Self)
	}
}

impl Accept for UdsAccept {
	type Conn = UnixStream;
	type Error = std::io::Error;

	fn poll_accept(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
		let (stream, _addr) = ready!(self.0.poll_accept(cx))?;
		Poll::Ready(Some(Ok(stream)))
	}
}

#[derive(Clone, Debug)]
struct UdsConnectInfo {
	peer_addr: Arc<SocketAddr>,
	peer_cred: UCred,
}

impl Connected<&UnixStream> for UdsConnectInfo {
	fn connect_info(target: &UnixStream) -> Self {
		let peer_addr = target.peer_addr().unwrap();
		let peer_cred = target.peer_cred().unwrap();

		Self {
			peer_addr: Arc::new(peer_addr),
			peer_cred,
		}
	}
}

pub async fn run(app: Router, addr: &BindableAddr) -> Result<(), Error> {
	match addr {
		BindableAddr::Tcp(socket_addr) => {
			axum::Server::bind(socket_addr)
				.serve(app.into_make_service())
				.await
		}
		BindableAddr::Unix(path) => {
			let incoming = UdsAccept::new(path).map_err(|err| Error::BindUnix(err, path.clone()))?;
			axum::Server::builder(incoming)
				.serve(app.into_make_service_with_connect_info::<UdsConnectInfo>())
				.await
		}
	}
	.map_err(Error::RunServer)
}
