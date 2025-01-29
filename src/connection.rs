use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub enum ListenAddr {
    IP(SocketAddr),
}