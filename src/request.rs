use core::fmt;
use std::io::{self, Cursor, Error as IoError, ErrorKind};
use std::str::FromStr;
use std::{io::{Read, Write}, net::SocketAddr, sync::mpsc::Sender};

use chunked_transfer::Decoder;

use crate::common::{HTTPVersion, Header, Method, StatusCode};
use crate::response::{self, Response};
use crate::util::equal_reader::EqualReader;
use crate::util::fused_reader::FusedReader;

pub struct Request {

    data_reader: Option<Box<dyn Send + Read + 'static>>,

    response_writer: Option<Box<dyn Send + Write + 'static>>,

    remote_addr: Option<SocketAddr>,

    secure: bool,

    method: Method,

    path: String,

    http_version: HTTPVersion,

    headers: Vec<Header>,

    body_length: Option<usize>,

    must_send_continue: bool,

    notify_when_responded: Option<Sender<()>>,
}

struct NotifyOnDrop<R> {
    sender: Sender<()>,
    inner: R
}

impl<R: Read> Read for  NotifyOnDrop<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Write> Write for NotifyOnDrop<R> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<R> Drop for NotifyOnDrop<R> {
    fn drop(&mut self) {
        self.sender.send(()).unwrap();
    }
}

#[derive(Debug)]
pub enum RequestCreationError {
    ExpectationFailed,
    CreationIoError(IoError),
}

impl From<IoError> for RequestCreationError {
    fn from(value: IoError) -> Self {
        RequestCreationError::CreationIoError(value)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn new_request<R, W>(
    secure: bool,
    method: Method,
    path: String,
    version: HTTPVersion,
    headers: Vec<Header>,
    remote_addr: Option<SocketAddr>,
    mut source_data: R,
    writer: W
) -> Result<Request, RequestCreationError> 
where
    R: Read + Send + 'static,
    W: Write + Send + 'static
{
    let transfer_encoding = headers
        .iter()
        .find(|h| h.field.equiv("Transfer-Encoding"))
        .map(|h| h.value.clone());

    let content_length = if transfer_encoding.is_some() {
        None
    } else {
        headers
            .iter()
            .find(|h| h.field.equiv("Content-Length"))
            .and_then(|h| FromStr::from_str(h.value.as_str()).ok())
    };

    let expects_continue = {
        match headers
            .iter()
            .find(|h| h.field.equiv("Expect"))
            .map(|h| h.value.as_str()) {
                None => false,
                Some(v) if v.eq_ignore_ascii_case("100-continue") => true,
                _ => return Err(RequestCreationError::ExpectationFailed)
            }
    };

    let connection_upgrade = {
        match headers
            .iter()
            .find(|h| h.field.equiv("Connection"))
            .map(|h| h.value.as_str()) {
                Some(v) if v.to_ascii_lowercase().contains("upgrade") => true,
                _ => false
            }
    };

    let reader = if connection_upgrade {
        Box::new(source_data) as Box<dyn Send + Read + 'static>
    } else if let Some(content_length) = content_length{
        if content_length == 0 {
            Box::new(io::empty()) as Box<dyn Read + Send + 'static>
        } else if content_length <= 1024 && !expects_continue {

            let mut buffer = vec![0u8; 1024];
            let mut offset = 0;

            while offset != content_length {
                let read = source_data.read(&mut buffer[offset..])?;

                if read == 0 {
                    // the socket returned EOF, but we were before the expected content-length
                    // aborting
                    let info = "Connection has been closed before we received enough data";
                    let err = IoError::new(ErrorKind::ConnectionAborted, info);
                    return Err(RequestCreationError::CreationIoError(err));
                }
                offset += read;
            }
            Box::new(Cursor::new(buffer)) as Box<dyn Read + Send + 'static>
        } else {
            let (data_reader, _) = EqualReader::new(source_data, content_length); // TODO:
            Box::new(FusedReader::new(data_reader)) as Box<dyn Read + Send + 'static>
        }


    } else if transfer_encoding.is_some() {
                // if a transfer-encoding was specified, then "chunked" is ALWAYS applied
        // over the message (RFC2616 #3.6)
        Box::new(FusedReader::new(Decoder::new(source_data))) as Box<dyn Read + Send + 'static>
    } else {
        Box::new(io::empty()) as Box<dyn Read + Send + 'static>
    };
    Ok(Request {
        data_reader: Some(reader),
        response_writer: Some(Box::new(writer) as Box<dyn Write + Send + 'static>),
        remote_addr,
        secure,
        method,
        path,
        http_version: version,
        headers,
        body_length: content_length,
        must_send_continue: expects_continue,
        notify_when_responded: None,
    })
}

impl Request {
    
    #[inline]
    pub fn secure(&self) -> bool {
        self.secure
    }

    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    #[inline]
    pub fn url(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    #[inline]
    pub fn http_version(&self) -> &HTTPVersion {
        &self.http_version
    }

    #[inline]
    pub fn body_length(&self) -> Option<usize> {
        self.body_length
    }

    #[inline]
    pub fn remote_addr(&self) -> Option<&SocketAddr> {
        self.remote_addr.as_ref()
    }

    pub fn ungrade<R: Read>(
        mut self,
        protocol: &str,
        response: Response<R>
    ) -> Box<dyn ReadWrite + Send> {
        todo!()
    }

    #[inline]
    pub fn as_reader(&mut self) -> &mut dyn Read {
        if self.must_send_continue {
            let msg = Response::empty(StatusCode(100));
            msg.raw_print(
                self.response_writer.as_mut().unwrap().by_ref(),
                self.http_version.clone(),
                &self.headers,
                true,
                None
            )
            .ok();
            self.response_writer.as_mut().unwrap().flush().ok();
            self.must_send_continue = false;
        }
        self.data_reader.as_mut().unwrap()
    }

    #[inline]
    pub fn as_writer(&mut self) -> Box<dyn Write + Send + 'static> {
        todo!()
    }

    fn extract_writer_impl(&mut self) -> Box<dyn Write + Send + 'static> {
        use std::mem;

        assert!(self.response_writer.is_some());

        let mut writer = None;
        mem::swap(&mut self.response_writer, &mut writer);
        writer.unwrap()
    }

    fn extract_reader_impl(&mut self) -> Box<dyn Read + Send + 'static> {
        use std::mem;

        assert!(self.data_reader.is_some());

        let mut reader = None;
        mem::swap(&mut self.data_reader, &mut reader);
        reader.unwrap()
    }

    #[inline]
    pub fn respond<R>(&mut self, response: Response<R>) -> Result<(), IoError>
    where 
        R: Read 
    {
        let res = self.respond_impl(response);
        if let Some(sender) = self.notify_when_responded.take() {
            sender.send(()).unwrap();
        }
        res
    }

    fn respond_impl<R>(&mut self, response: Response<R>) -> Result<(), IoError>
    where
        R: Read 
    {
        let mut writer = self.extract_writer_impl();
        let do_not_send_body = self.method().eq(&Method::Head);

        Self::ignore_client_closing_errors(response.raw_print(
            writer.by_ref(), 
            self.http_version.clone(), 
            &self.headers, 
            do_not_send_body, 
            None
        ))?;

        Self::ignore_client_closing_errors(writer.flush())
    }

    fn ignore_client_closing_errors(result: io::Result<()>) -> io::Result<()> {
        result.or_else(|e| match e.kind() {
            ErrorKind::BrokenPipe 
            | ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionRefused 
            | ErrorKind::ConnectionReset => Ok(()),
            _ => Err(e)
        })
    }

    pub(crate) fn with_notify_sender(mut self, sender: Sender<()>) -> Self {
        self.notify_when_responded = Some(sender);
        self
    }

}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Request({} {} from {:?})",
            self.method,
            self.path,
            self.remote_addr
        )
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        if self.response_writer.is_some() {
            let response = Response::empty(StatusCode(500));
            let _ = self.respond(response);
            if let Some(sender) = self.notify_when_responded.take() {
                sender.send(()).unwrap();
            }
        }
    }
}

pub trait ReadWrite: Read + Write {}

impl<T> ReadWrite for T
where T: Read + Write
{
    
}
