use std::{net::TcpStream, thread, time::Duration};


pub fn new_one_client_one_server() -> (my_tiny_http::Server, TcpStream) {
    let server = my_tiny_http::Server::http("0.0.0.0:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    (server, client)
}

pub fn new_client_to_hello_world_server() -> TcpStream {
    let server = my_tiny_http::Server::http("0.0.0.0:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let client = TcpStream::connect(("127.0.0.1", port)).unwrap();

    thread::spawn(move || {
        let mut cycles = 3 * 1000 / 20;

        loop {
            if let Some(mut rq) = server.try_recv().unwrap() {
                let response = my_tiny_http::Response::from_string("hello world".to_string());
                rq.respond(response).unwrap();
            }

            thread::sleep(Duration::from_millis(20));

            cycles -= 1;
            if cycles == 0 {
                break;
            }
        }
    });
    client
}