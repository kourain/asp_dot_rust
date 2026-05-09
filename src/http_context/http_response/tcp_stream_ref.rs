use std::ops::{Deref, DerefMut};

use tokio::{io::WriteHalf, net::TcpStream};

pub struct TcpWriteStreamRef(*mut WriteHalf<TcpStream>);

unsafe impl Send for TcpWriteStreamRef {}
unsafe impl Sync for TcpWriteStreamRef {}

impl TcpWriteStreamRef {
    pub fn new(write_stream: &mut WriteHalf<TcpStream>) -> Self {
        Self(write_stream as *mut WriteHalf<TcpStream>)
    }
}

impl Deref for TcpWriteStreamRef {
    type Target = WriteHalf<TcpStream>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for TcpWriteStreamRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}