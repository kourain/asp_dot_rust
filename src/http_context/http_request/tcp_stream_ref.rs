use std::ops::{Deref, DerefMut};

use tokio::{io::ReadHalf, net::TcpStream};

pub struct TcpReadStreamRef(*mut ReadHalf<TcpStream>);

unsafe impl Send for TcpReadStreamRef {}
unsafe impl Sync for TcpReadStreamRef {}

impl TcpReadStreamRef {
    pub fn new(read_stream: &mut ReadHalf<TcpStream>) -> Self {
        Self(read_stream as *mut ReadHalf<TcpStream>)
    }
}

impl Deref for TcpReadStreamRef {
    type Target = ReadHalf<TcpStream>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for TcpReadStreamRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}