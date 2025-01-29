use std::sync::{atomic::AtomicBool, Arc};
use std::io::Error as IoError;
use util::messages_queue::MessageQueue;
use connection::ListenAddr;


pub use request::Request;

mod request;
mod util;
mod connection;

pub struct Server {

    //只要服务还存在就为false
    //设置为true时，所有子任务将在几百毫秒内关闭
    close: Arc<AtomicBool>,

    //子线程的消息队列
    messages: Arc<MessageQueue<Message>>,

    //result for TcpListener::local_addr()
    listeninng_port: ListenAddr
}

enum Message {
    Error(IoError),
    NewRequest(Request)
}

impl From<IoError> for Message {
    fn from(e: IoError) -> Self {
        Message::Error(e)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

}
