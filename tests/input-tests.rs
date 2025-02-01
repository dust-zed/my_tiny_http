extern crate my_tiny_http;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::sync::mpsc;
use std::thread;

#[allow(dead_code)]
mod support;

#[test]
fn basic_string_input() {

    let (server, client) = support::new_one_client_one_server();

    {
        let mut client = client;
        (write!(client, "GET / HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: 5\r\n\r\nhello")).unwrap();
    }
    let mut request = server.recv().unwrap();

    let mut output = String::new();
    request.as_reader().read_to_string(&mut output).unwrap();
    assert_eq!("hello", output);
}

#[test]
fn wrong_content_length() {
    let (server, client) = support::new_one_client_one_server();

    {
        let mut client = client;
        (write!(client, "GET / HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain; charset=utf8\r\nContent-Length: 3\r\n\r\nhello")).unwrap();
    }

    let mut request = server.recv().unwrap();

    let mut output = String::new();
    //request.as_reader().read_to_string(&mut output).unwrap();
    assert_eq!(output, "hel");
}