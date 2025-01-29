use connection::{ConfigListenAddr, ListenAddr, Listener};
use std::error::Error;
use std::io::Error as IoError;
use std::net::ToSocketAddrs;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;
use util::messages_queue::MessageQueue;

pub use request::Request;

mod connection;
mod request;
mod util;
mod log;
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

    }

    ///builds a new server that listens on specific address
    pub fn new(config: ServerConfig) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        let listener = config.addr.bind()?;
        Self::f
    }

    /// builds a new server using specified TCP listener.
    ///
    /// This is useful if you've constructed TcpListener using some less usual method
    /// such as from systemd. For ohter cases, you probably want `new` function.
    pub fn from_listener<L: Into<Listener>>(
        listener: L,
        ssl_config: Option<SslConfig>
    ) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        let listener = listener.into();
        //building the "close" variable
        let close_trigger = Arc::new(AtomicBool::new(false));

        //building the TcpListener
        let (server, local_addr) = {
            let local_addr = listener.local_addr()?;
            log::debug!("Server listening on {}", local_addr);
            (listener, local_addr)
        };

        //building the SSL capabilities
        //todo!()

        // creating a task where server.accept() is continuously called
        // and ClientConnection objects are pushed in the messages queue
        let messages = MessageQueue::with_capacity(8);
        let inside_close_trigger =  close_trigger.clone();
        let inside_messages = messages.clone();
        thread::spawn(move || {
            // a tasks pool is used to dispatch the connections into threads
            
        });
        Ok(())
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
