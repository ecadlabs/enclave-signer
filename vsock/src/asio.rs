use crate::{Datagram as SyncDatagram, Listener as SyncListener, SocketAddr, Stream as SyncStream};
use std::io::{Error, ErrorKind, Result};
use std::net::Shutdown;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncWrite, Interest, ReadBuf, Ready};

pub struct Datagram(AsyncFd<SyncDatagram>);

impl Datagram {
    pub fn unbound() -> Result<Self> {
        let inner = SyncDatagram::unbound()?;
        inner.set_nonblocking(true)?;
        Ok(Self(AsyncFd::new(inner)?))
    }

    pub fn bind(addr: &SocketAddr) -> Result<Self> {
        let inner = SyncDatagram::bind(addr)?;
        inner.set_nonblocking(true)?;
        Ok(Self(AsyncFd::new(inner)?))
    }

    pub fn connect(&self, addr: &SocketAddr) -> Result<()> {
        self.0.get_ref().connect(addr)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.get_ref().local_addr()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.get_ref().peer_addr()
    }

    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.0
            .async_io(Interest::READABLE, |inner| inner.recv(buf))
            .await
    }

    pub async fn send(&self, buf: &[u8]) -> Result<usize> {
        self.0
            .async_io(Interest::WRITABLE, |inner| inner.send(buf))
            .await
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.0
            .async_io(Interest::READABLE, |inner| inner.recv_from(buf))
            .await
    }

    pub async fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> Result<usize> {
        self.0
            .async_io(Interest::WRITABLE, |inner| inner.send_to(buf, addr))
            .await
    }

    pub fn shutdown_sync(&self, how: Shutdown) -> Result<()> {
        self.0.get_ref().shutdown(how)
    }

    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.get_ref().take_error()
    }
}

fn poll_read<F, T>(
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
    recv_fn: F,
    poller: &AsyncFd<T>,
) -> Poll<Result<()>>
where
    T: AsRawFd,
    F: Fn(&mut [u8]) -> Result<usize>,
{
    // loop may not be necessary but this is how the similar piece is implemented in Tokio
    loop {
        let mut guard = ready!(poller.poll_read_ready(cx))?;
        let b = buf.initialize_unfilled();
        match recv_fn(b) {
            Ok(n) => {
                if n > 0 && n < b.len() {
                    guard.clear_ready_matching(Ready::READABLE);
                }
                buf.advance(n);
                break Poll::Ready(Ok(()));
            }
            Err(err) => {
                if err.kind() == ErrorKind::WouldBlock {
                    guard.clear_ready_matching(Ready::READABLE);
                } else {
                    break Poll::Ready(Err(err));
                }
            }
        }
    }
}

fn poll_write<F, T>(
    cx: &mut Context<'_>,
    buf: &[u8],
    send_fn: F,
    poller: &AsyncFd<T>,
) -> Poll<Result<usize>>
where
    T: AsRawFd,
    F: Fn(&[u8]) -> Result<usize>,
{
    loop {
        let mut guard = ready!(poller.poll_write_ready(cx))?;
        match send_fn(buf) {
            Ok(n) => {
                if n > 0 && n < buf.len() {
                    guard.clear_ready_matching(Ready::WRITABLE);
                }
                break Poll::Ready(Ok(n));
            }
            Err(err) => {
                if err.kind() == ErrorKind::WouldBlock {
                    guard.clear_ready_matching(Ready::WRITABLE);
                } else {
                    break Poll::Ready(Err(err));
                }
            }
        }
    }
}

impl AsyncRead for Datagram {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        poll_read(cx, buf, |buf| self.0.get_ref().recv(buf), &self.0)
    }
}

impl AsyncWrite for Datagram {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        poll_write(cx, buf, |buf| self.0.get_ref().send(buf), &self.0)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        self.shutdown_sync(Shutdown::Write)?;
        Poll::Ready(Ok(()))
    }
}

impl AsFd for Datagram {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for Datagram {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

pub struct Stream(AsyncFd<SyncStream>);

fn is_in_progress(err: &Error) -> bool {
    match err.raw_os_error() {
        Some(code) => match code {
            libc::EINPROGRESS | libc::EALREADY => true,
            _ => false,
        },
        None => false,
    }
}

impl Stream {
    pub async fn connect(addr: &SocketAddr) -> Result<Self> {
        let inner = SyncStream::unbound()?;
        inner.set_nonblocking(true)?;

        let sock = Stream(AsyncFd::new(inner)?);

        match sock.0.get_ref().connect_to_addr(addr) {
            Ok(()) => Ok(sock),
            Err(err) if is_in_progress(&err) => loop {
                let _ = sock.0.writable().await?;
                if let Some(err) = sock.0.get_ref().take_error()? {
                    break Err(err);
                }
                if let Ok(_) = sock.0.get_ref().peer_addr() {
                    break Ok(sock);
                }
            },
            Err(err) => Err(err),
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.get_ref().local_addr()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.get_ref().peer_addr()
    }

    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.0
            .async_io(Interest::READABLE, |inner| inner.recv(buf))
            .await
    }

    pub async fn send(&self, buf: &[u8]) -> Result<usize> {
        self.0
            .async_io(Interest::WRITABLE, |inner| inner.send(buf))
            .await
    }

    pub fn shutdown_sync(&self, how: Shutdown) -> Result<()> {
        self.0.get_ref().shutdown(how)
    }

    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.get_ref().take_error()
    }
}

impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        poll_read(cx, buf, |buf| self.0.get_ref().recv(buf), &self.0)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        poll_write(cx, buf, |buf| self.0.get_ref().send(buf), &self.0)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
        self.shutdown_sync(Shutdown::Write)?;
        Poll::Ready(Ok(()))
    }
}

impl AsFd for Stream {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for Stream {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

pub struct Listener(AsyncFd<SyncListener>);

impl Listener {
    pub fn bind(addr: &SocketAddr) -> Result<Self> {
        let inner = SyncListener::bind(addr)?;
        inner.set_nonblocking(true)?;
        Ok(Self(AsyncFd::new(inner)?))
    }

    pub async fn accept(&self) -> Result<(Stream, SocketAddr)> {
        let (stream, addr) = self
            .0
            .async_io(Interest::READABLE, |inner| inner.accept())
            .await?;
        stream.set_nonblocking(true)?;
        Ok((Stream(AsyncFd::new(stream)?), addr))
    }

    pub fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<Result<(Stream, SocketAddr)>> {
        loop {
            let mut guard = ready!(self.0.poll_read_ready(cx))?;
            match self.0.get_ref().accept() {
                Ok((stream, addr)) => {
                    stream.set_nonblocking(true)?;
                    break Poll::Ready(Ok((Stream(AsyncFd::new(stream)?), addr)));
                }
                Err(err) => {
                    if err.kind() == ErrorKind::WouldBlock {
                        guard.clear_ready_matching(Ready::READABLE);
                    } else {
                        break Poll::Ready(Err(err));
                    }
                }
            }
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.get_ref().local_addr()
    }
}

impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for Listener {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
