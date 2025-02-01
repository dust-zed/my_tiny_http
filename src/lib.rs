use connection::Connection;
#[cfg(any(
    feature = "ssl-openssl",
    feature = "ssl-rustls",
    feature = "ssl-native-tls"
))]
use zeroize::Zeroizing;

use client::ClientConnection;
use connection::{ConfigListenAddr, ListenAddr, Listener};
use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::io::Result as IoResult;
use std::net::Shutdown;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;
use std::time::Duration;
use util::messages_queue::MessageQueue;

pub use request::Request;

mod client;
mod common;
mod connection;
mod log;
mod request;
mod response;
mod ssl;
mod util;
pub struct Server {
    //只要服务还存在就为false
    //设置为true时，所有子任务将在几百毫秒内关闭
    close: Arc<AtomicBool>,

    //子线程的消息队列
    messages: Arc<MessageQueue<Message>>,

    //result for TcpListener::local_addr()
    listeninng_port: ListenAddr,
}

enum Message {
    Error(IoError),
    NewRequest(Request),
}

impl From<IoError> for Message {
    fn from(e: IoError) -> Self {
        Message::Error(e)
    }
}

impl From<Request> for Message {
    fn from(req: Request) -> Self {
        Message::NewRequest(req)
    }
}

pub struct IncomingRequests<'a> {
    server: &'a Server,
}

impl Server {
    pub fn http<A>(addr: A) -> Result<Server, Box<dyn Error + Send + Sync + 'static>>
    where
        A: ToSocketAddrs,
    {
        Server::new(ServerConfig {
            addr: ConfigListenAddr::from_socket_addrs(addr)?,
            ssl: None
        })
    }

    ///builds a new server that listens on specific address
    pub fn new(config: ServerConfig) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        let listener = config.addr.bind()?;
        Self::from_listener(listener, config.ssl)
    }

    /// builds a new server using specified TCP listener.
    ///
    /// This is useful if you've constructed TcpListener using some less usual method
    /// such as from systemd. For ohter cases, you probably want `new` function.
    pub fn from_listener<L: Into<Listener>>(
        listener: L,
        ssl_config: Option<SslConfig>,
    ) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        let listener: Listener = listener.into();
        //building the "close" variable
        let close_trigger = Arc::new(AtomicBool::new(false));

        //building the TcpListener
        let (server, local_addr) = {
            let local_addr = listener.local_addr()?;
            log::debug!("Server listening on {}", local_addr);
            (listener, local_addr)
        };

        //building the SSL capabilities
        //todo!() 暂时是copy来的
        #[cfg(any(
            all(feature = "ssl-openssl", feature = "ssl-rustls"),
            all(feature = "ssl-openssl", feature = "ssl-native-tls"),
            all(feature = "ssl-native-tls", feature = "ssl-rustls"),
        ))]

        compile_error!(
            "Only one feature from 'ssl-openssl', 'ssl-rustls', 'ssl-native-tls' can be enabled at the same time"
        );
        #[cfg(not(any(
            feature = "ssl-openssl",
            feature = "ssl-rustls",
            feature = "ssl-native-tls"
        )))]
        type SslContext = ();

        #[cfg(any(
            feature = "ssl-openssl",
            feature = "ssl-rustls",
            feature = "ssl-native-tls"
        ))]
        type SslContext = crate::ssl::SslContextImpl;
        let ssl: Option<SslContext> = {
            match ssl_config {
                #[cfg(any(
                    feature = "ssl-openssl",
                    feature = "ssl-rustls",
                    feature = "ssl-native-tls"
                ))]
                Some(config) => Some(SslContext::from_pem(
                    config.certificate,
                    Zeroizing::new(config.private_key),
                )?),
                #[cfg(not(any(
                    feature = "ssl-openssl",
                    feature = "ssl-rustls",
                    feature = "ssl-native-tls"
                )))]
                Some(_) => return Err(
                    "Building a server with SSL requires enabling the `ssl` feature in tiny-http"
                        .into(),
                ),
                None => None,
            }
        };

        // creating a task where server.accept() is continuously called
        // and ClientConnection objects are pushed in the messages queue
        let messages = MessageQueue::with_capacity(8);
        let inside_close_trigger = close_trigger.clone();
        let inside_messages = messages.clone();
        thread::spawn(move || {
            // a tasks pool is used to dispatch the connections into threads
            let task_pool = util::task_pool::TaskPool::new();

            log::debug!("Running accept thread");
            while !inside_close_trigger.load(Ordering::Relaxed) {
                let new_client = match server.accept() {
                    Ok((sock, _)) => {
                        use util::refined_tcp_stream::RefinedTcpStream;
                        let (read_closable, write_closble) = match ssl {
                            None => RefinedTcpStream::new(sock),
                            #[cfg(any(
                                feature = "ssl-openssl",
                                feature = "ssl-rustls",
                                feature = "ssl-native-tls"
                            ))]
                            Some(ref ssl) => {
                                // trying to apply SSL over the connection
                                // if an error occurs, we just close the socket and resume listening
                                let sock = match ssl.accept(sock) {
                                    Ok(s) => s,
                                    Err(_) => continue,
                                };

                                RefinedTcpStream::new(sock)
                            }
                            #[cfg(not(any(
                                feature = "ssl-openssl",
                                feature = "ssl-rustls",
                                feature = "ssl-native-tls"
                            )))]
                            Some(ref _ssl) => unreachable!(),
                        };
                        Ok(ClientConnection::new(write_closble, read_closable))
                    }
                    Err(e) => Err(e),
                };

                match new_client {
                    Ok(client) => {
                        let messages = inside_messages.clone();
                        let mut client = Some(client);
                        task_pool.spawn(Box::new(move || {
                            if let Some(client) = client.take() {
                                if client.secure() {
                                    let (sender, receiver) = mpsc::channel();
                                    for rq in client {
                                        messages.push(rq.with_notify_sender(sender.clone()).into());
                                        receiver.recv().unwrap();
                                    }
                                } else {
                                    for rq in client {
                                        messages.push(rq.into());
                                    }
                                }
                            }
                        }));
                    }
                    Err(e) => {
                        log::error!("Error accepting new client: {}", e);
                        inside_messages.push(e.into());
                        break;
                    }
                }
            }
        });

        Ok(Server {
            close: close_trigger,
            messages,
            listeninng_port: local_addr,
        })
    }

    #[inline]
    pub fn incoming_requests(&self) -> IncomingRequests<'_> {
        IncomingRequests { server : self}
    }

    #[inline]
    pub fn server_addr(&self) -> ListenAddr {
        self.listeninng_port.clone()
    }

    #[inline]
    pub fn num_connections(&self) -> usize {
        unimplemented!()
    }

    pub fn recv(&self) -> IoResult<Request> {
        match self.messages.pop() {
            Some(Message::Error(err)) => Err(err),
            Some(Message::NewRequest(rq)) => Ok(rq),
            None => Err(IoError::new(IoErrorKind::Other, "thread unblocked")),
        }
    }

    /// Same as `recv()` but doesn't block longer than timeout
    pub fn recv_timeout(&self, timeout: Duration) -> IoResult<Option<Request>> {
        match self.messages.pop_timeout(timeout) {
            Some(Message::Error(err)) => Err(err),
            Some(Message::NewRequest(rq)) => Ok(Some(rq)),
            None => Ok(None),
        }
    }

    /// Same as `recv()` but doesn't block.
    pub fn try_recv(&self) -> IoResult<Option<Request>> {
        match self.messages.try_pop() {
            Some(Message::Error(err)) => Err(err),
            Some(Message::NewRequest(rq)) => Ok(Some(rq)),
            None => Ok(None),
        }
    }

    /// Unblock thread stuck in recv() or incoming_requests().
    /// If there are several such threads, only one is unblocked.
    /// This method allows graceful shutdown of server.
    pub fn unblock(&self) {
        self.messages.unblock();
    }
}

impl Iterator for IncomingRequests<'_> {
    type Item = Request;
    fn next(&mut self) -> Option<Request> {
        self.server.recv().ok()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.close.store(true, Ordering::Relaxed);
        // Connect briefly to ourselves to unblock the accept thread
        let maybe_stream = match &self.listeninng_port {
            ListenAddr::IP(addr) => TcpStream::connect(addr).map(Connection::from),
            #[cfg(unix)]
            ListenAddr::Unix(addr) => {
                // TODO: use connect_addr when its stabilized.
                let path = addr.as_pathname().unwrap();
                std::os::unix::net::UnixStream::connect(path).map(Connection::from)
            }
        };
        if let Ok(stream) = maybe_stream {
            let _ = stream.shutdown(Shutdown::Both);
        }

        #[cfg(unix)]
        if let ListenAddr::Unix(addr) = &self.listeninng_port {
            if let Some(path) = addr.as_pathname() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}


pub struct ServerConfig {
    //the address try to listen to
    pub addr: ConfigListenAddr,

    //if `Some``, then the server will use SSL to encode the communications
    pub ssl: Option<SslConfig>,
}

pub struct SslConfig {
    ///contatains the public certifications to send to client
    pub certificate: Vec<u8>,
    ///contains the ultra-secret private key used to decode communications
    pub private_key: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
}
