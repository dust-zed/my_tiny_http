use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::io::{BufReader, BufWriter, ErrorKind, Read};
use std::net::SocketAddr;
use std::str::FromStr;

use ascii::AsciiString;

use crate::common::Method;
use crate::response;
use crate::Request;
use crate::{
    common::HTTPVersion,
    util::{
        refined_tcp_stream::RefinedTcpStream,
        sequential::{SequentialReader, SequentialReaderBuilder, SequentialWriterBuilder},
    },
};

pub struct ClientConnection {
    remote_addr: IoResult<Option<SocketAddr>>,

    source: SequentialReaderBuilder<BufReader<RefinedTcpStream>>,

    sink: SequentialWriterBuilder<BufWriter<RefinedTcpStream>>,

    next_header_source: SequentialReader<BufReader<RefinedTcpStream>>,

    no_more_request: bool,

    secure: bool,
}

/// Error that can happen when reading a request
#[derive(Debug)]
enum ReadError {
    WrongRequestLine,
    WrongHeader(HTTPVersion),
    ExpectationFailed(HTTPVersion),
    ReadIoError(IoError),
}

impl ClientConnection {
    pub fn new(
        write_socket: RefinedTcpStream,
        mut read_socket: RefinedTcpStream,
    ) -> ClientConnection {
        let remote_addr = read_socket.peer_addr();
        let secure = read_socket.secure();

        let mut source = SequentialReaderBuilder::new(BufReader::with_capacity(1024, read_socket));
        let first_header = source.next().unwrap();

        ClientConnection {
            source,
            sink: SequentialWriterBuilder::new(BufWriter::with_capacity(1024, write_socket)),
            remote_addr,
            next_header_source: first_header,
            no_more_request: false,
            secure,
        }
    }

    pub fn secure(&self) -> bool {
        self.secure
    }

    fn read_next_line(&mut self) -> IoResult<AsciiString> {
        let mut buf = Vec::new();
        let mut pre_byte_was_cr = false;

        loop {
            let byte = self.next_header_source.by_ref().bytes().next();
            let byte = match byte {
                Some(b) => b?,
                None => return Err(IoError::new(ErrorKind::ConnectionAborted, "Unexpected EOF")),
            };

            if byte == b'\n' && pre_byte_was_cr {
                buf.pop();
                return AsciiString::from_ascii(buf)
                    .map_err(|_| IoError::new(ErrorKind::InvalidInput, "Header is not in ASCII"));
            }

            pre_byte_was_cr = byte == b'\r';
            buf.push(byte);
        }
    }

    fn read(&mut self) -> Result<Request, ReadError> {
        let (method, path, version, headers) = {
            let (method, path, version) = {
                let line = self.read_next_line().map_err(ReadError::ReadIoError)?;
                parse_request_line(line.as_str().trim())?
            };

            let headers = {
                let mut headers = Vec::new();
                loop {
                    let line = self.read_next_line().map_err(ReadError::ReadIoError)?;

                    if line.is_empty() {
                        break;
                    }

                    headers.push(match FromStr::from_str(line.as_str().trim()) {
                        Ok(h) => h,
                        _ => return Err(ReadError::WrongHeader(version)),
                    });
                }
                headers
            };
            (method, path, version, headers)
        };

        let writer = self.sink.next().unwrap();

        let mut data_source = self.source.next().unwrap();
        std::mem::swap(&mut self.next_header_source, &mut data_source);

        let request = crate::request::new_request(
            self.secure, 
            method,
            path, 
            version.clone(),
            headers, 
            *self.remote_addr.as_ref().unwrap(),
            data_source,
            writer,
        )
        .map_err(|e| {
            use crate::request;
            match e {
                request::RequestCreationError::CreationIoError(e) => ReadError::ReadIoError(e),
                request::RequestCreationError::ExpectationFailed => {
                    ReadError::ExpectationFailed(version)
                }
            }
        })?;
        Ok(request)
    }
}

impl Iterator for ClientConnection {
    type Item = Request;
    
    fn next(&mut self) -> Option<Self::Item> {
       use crate::response::Response;
       use crate::common::StatusCode;

       if self.no_more_request {
        return None;
       }

       loop {
            let rq = match self.read() {
                Err(ReadError::WrongRequestLine) => {
                    let writer = self.sink.next().unwrap();
                    let response = Response::empty(StatusCode(400));
                    response
                        .raw_print(writer, HTTPVersion(1, 1), &[], false , None)
                        .ok();
                    return None;
                }

                Err(ReadError::WrongHeader(ver)) => {
                    let writer = self.sink.next().unwrap();
                    let response = Response::empty(StatusCode(400));
                    response
                        .raw_print(writer, ver, &[], false, None)
                        .ok();
                    return None;
                }

                Err(ReadError::ReadIoError(ref err)) if err.kind() == ErrorKind::TimedOut => {
                    let writer = self.sink.next().unwrap();
                    let response = Response::empty(StatusCode(408));
                    response
                        .raw_print(writer, HTTPVersion(1, 1), &[], false, None)
                        .ok();
                    return None;
                }

                Err(ReadError::ExpectationFailed(ver)) => {
                    let writer = self.sink.next().unwrap();
                    let response = Response::empty(StatusCode(407));
                    response
                        .raw_print(writer, ver, &[], true, None)
                        .ok();
                    return None;
                }

                Err(ReadError::ReadIoError(_)) => return None,

                Ok(rq) => rq,
            };

            if *rq.http_version() > (1, 1) {
                let writer = self.sink.next().unwrap();
                let response = Response::from_string(
                    "This server only supports HTTP versions 1.0 and 1.1 ".to_owned(),
                )
                .with_status_code(StatusCode(505));
                response
                    .raw_print(writer, HTTPVersion(1, 1), &[], false, None)
                    .ok();
                continue;
            }

                let connection_header = rq
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Connection"))
                    .map(|h| h.value.as_str());
                let lowercase = connection_header.map(|h| h.to_ascii_lowercase());

                match lowercase {
                    Some(ref val ) if val.contains("close") => self.no_more_request = true,
                    Some(ref val ) if val.contains("upgrade") => self.no_more_request = true,
                    Some(ref val )
                        if !val.contains("keep-alive") && *rq.http_version() == HTTPVersion(1, 0) => {
                            self.no_more_request = true;
                        }
                    None if *rq.http_version() == HTTPVersion(1, 0) => self.no_more_request = true,
                    _ => (), 
                };

                return Some(rq);
       }
    }

    
}

fn parse_http_version(version: &str) -> Result<HTTPVersion, ReadError> {
    let (major, minor) = match version {
        "HTTP/0.9" => (0u8, 9),
        "HTTP/1.0" => (1, 0),
        "HTTP/1.1" => (1, 1),
        "HTTP/2.0" => (2, 0),
        "HTTP/3.0" => (3, 0),
        _ => return Err(ReadError::WrongRequestLine),
    };
    Ok(HTTPVersion(major, minor))
}
fn parse_request_line(line: &str) -> Result<(Method, String, HTTPVersion), ReadError> {
    let mut parts = line.split(' ');

    let method = parts.next().and_then(|w| w.parse().ok());
    let path = parts.next().map(ToOwned::to_owned);
    let http_version = parts.next().and_then(|w| parse_http_version(w).ok());
    method
        .and_then(|method| Some((method, path?, http_version?)))
        .ok_or(ReadError::WrongRequestLine)
}
