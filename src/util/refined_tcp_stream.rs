use std::{io::Result as IoResult, net::{Shutdown, SocketAddr}};
use crate::connection::Connection;
#[cfg(any(
    feature = "ssl-openssl",
    feature = "ssl-rustls",
    feature = "ssl-native-tls"
))]
use crate::ssl::SslStream;

pub(crate) enum Stream {
    Http(Connection),

    #[cfg(any(
        feature = "ssl-openssl",
        feature = "ssl-rustls",
        feature = "ssl-native-tls"
    ))]
    Https(SslStream),
}

impl Clone for Stream {
    fn clone(&self) -> Self {
        match self {
            Stream::Http(tcp_stream) => Stream::Http(tcp_stream.try_clone().unwrap()),
            #[cfg(any(
                feature = "ssl-openssl",
                feature = "ssl-rustls",
                feature = "ssl-native-tls"
            ))]
            Stream::Https(ssl_stream) => Stream::Https(ssl_stream.clone())
        }
    }
}

impl From<Connection> for Stream {
    fn from(tcp_stream: Connection) -> Self {
        Stream::Http(tcp_stream)
    }
}

impl Stream {
    fn secure(&self) -> bool {
        match self {
            Stream::Http(_) => false,
            #[cfg(any(
                feature = "ssl-openssl",
                feature = "ssl-rustls",
                feature = "ssl-native-tls"
            ))]
            Stream::Https(_) => true,
        }
    }

    fn peer_addr(&mut self) -> IoResult<Option<SocketAddr>> {
        match self {
            Stream::Http(tcp_stream) => tcp_stream.peer_addr(),
            #[cfg(any(
                feature = "ssl-openssl",
                feature = "ssl-rustls",
                feature = "ssl-native-tls"
            ))]
            Stream::Https(ssl_stream) => ssl_stream.peer_addr()
        }
    }

    fn shutdown(&mut self, how: Shutdown) -> IoResult<()> {
        match self {
            Stream::Http(tcp_stream) => tcp_stream.shutdown(how),
            #[cfg(any(
                feature = "ssl-openssl",
                feature = "ssl-rustls",
                feature = "ssl-native-tls"
            ))]
            Stream::Https(ssl_stream) => ssl_stream.shutdown(how)
        }
    } 
}