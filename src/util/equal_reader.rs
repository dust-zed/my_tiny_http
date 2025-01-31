use std::{io::{Read, Result as IoResult}, sync::mpsc::{channel, Receiver, Sender}};

pub struct EqualReader<R> 
where
    R: Read 
{
    reader: R,
    size: usize,
    last_read_signal: Sender<IoResult<()>>
}

impl<R> EqualReader<R>
where 
    R: Read
{
    pub fn new(reader: R, size: usize) -> (Self, Receiver<IoResult<()>>) {
        let (tx, rx) = channel();

        let r = EqualReader {
            reader,
            size,
            last_read_signal: tx
        };

        (r, rx)
    } 

}

impl<R> Read for EqualReader<R>
where 
    R: Read
{
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if self.size == 0 {
            return Ok(0);
        }

        let buf = if buf.len() < self.size {
            buf
        } else {
            &mut buf[0..self.size]
        };

        match self.reader.read(buf) {
            Ok(len) => {
                self.size -= len;
                Ok(len)
            }
            err @ Err(_) => err
        }
    }
}

impl<R: Read> Drop for EqualReader<R>  {
    fn drop(&mut self) {
        
        let mut remaining_to_read = self.size;

        while remaining_to_read > 0 {
            let mut buf = vec![0; remaining_to_read];
            match self.read(&mut buf) {
                Err(e) => {
                    self.last_read_signal.send(Err(e)).ok();
                    break;
                }
                Ok(0) => {
                    self.last_read_signal.send(Ok(())).ok();
                    break;
                }
                Ok(other) => {
                    remaining_to_read -= other;
                }
            }
        }
    }
}