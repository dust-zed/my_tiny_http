use connection::{ConfigListenAddr, ListenAddr};
use std::error::Error;
use std::io::Error as IoError;
use std::net::ToSocketAddrs;
use std::sync::{atomic::AtomicBool, Arc};
use util::messages_queue::MessageQueue;

pub use request::Request;

mod connection;
mod request;
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

    }

    ///builds a new server that listens on specific address
    pub fn new(config: ServerConfig) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        let listener = config.addr.bind()?;
        
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
