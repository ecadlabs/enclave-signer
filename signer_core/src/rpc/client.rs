use crate::crypto::{KeyType, PublicKey, Signature};
use crate::rpc::{Error as RPCError, Request, Result as RPCResult};
use crate::{TryFromCBOR, TryIntoCBOR};
use serde::Serialize;
use std::io::{Read, Write};
use std::marker::PhantomData;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    RPC(RPCError),
    Serialize(ciborium::ser::Error<std::io::Error>),
    Deserialize(ciborium::de::Error<std::io::Error>),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
}

impl From<RPCError> for Error {
    fn from(value: RPCError) -> Self {
        Error::RPC(value)
    }
}

impl From<ciborium::de::Error<std::io::Error>> for Error {
    fn from(value: ciborium::de::Error<std::io::Error>) -> Self {
        Error::Deserialize(value)
    }
}

impl From<ciborium::ser::Error<std::io::Error>> for Error {
    fn from(value: ciborium::ser::Error<std::io::Error>) -> Self {
        Error::Serialize(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(error) => write!(f, "IO error: {}", error),
            Error::RPC(error) => write!(f, "RPC error: {}", error),
            Error::Serialize(error) => write!(f, "serialization error: {}", error),
            Error::Deserialize(error) => write!(f, "deserialization error: {}", error),
        }
    }
}

impl std::error::Error for Error {}

pub struct Client<T, C> {
    socket: T,
    _phantom: PhantomData<C>,
}

impl<T, C> Client<T, C>
where
    T: Read + Write,
    C: Serialize,
{
    pub fn new(sock: T) -> Self {
        Client {
            socket: sock,
            _phantom: PhantomData,
        }
    }

    fn round_trip<R>(&mut self, q: Request<C>) -> Result<R, Error>
    where
        R: TryFromCBOR,
        Request<C>: TryIntoCBOR,
        RPCResult<R>: TryFromCBOR,
        Error:
            From<<Request<C> as TryIntoCBOR>::Error> + From<<RPCResult<R> as TryFromCBOR>::Error>,
    {
        let buf = q.try_into_cbor()?;
        self.socket.write(&buf)?;

        let mut r_buf: [u8; 64 * 1024] = [0; 64 * 1024];
        let sz = self.socket.read(&mut r_buf)?;
        let res = RPCResult::<R>::try_from_cbor(&buf[0..sz])?;
        Ok(res?)
    }

    pub fn import(&mut self, key_data: &[u8]) -> Result<(PublicKey, usize), Error> {
        self.round_trip::<(PublicKey, usize)>(Request::Import(key_data.into()))
    }

    pub fn generate(&mut self, t: KeyType) -> Result<(Vec<u8>, PublicKey), Error> {
        self.round_trip::<(Vec<u8>, PublicKey)>(Request::Generate(t))
    }

    pub fn generate_and_import(
        &mut self,
        t: KeyType,
    ) -> Result<(Vec<u8>, PublicKey, usize), Error> {
        self.round_trip::<(Vec<u8>, PublicKey, usize)>(Request::GenerateAndImport(t))
    }

    pub fn try_sign(&mut self, handle: usize, msg: &[u8]) -> Result<Signature, Error> {
        self.round_trip::<Signature>(Request::Sign {
            handle: handle,
            msg: msg.into(),
        })
    }

    pub fn try_sign_with(&mut self, key_data: &[u8], msg: &[u8]) -> Result<Signature, Error> {
        self.round_trip::<Signature>(Request::SignWith {
            key_data: key_data.into(),
            msg: msg.into(),
        })
    }

    pub fn public_key(&mut self, handle: usize) -> Result<PublicKey, Error> {
        self.round_trip::<PublicKey>(Request::PublicKey(handle))
    }

    pub fn public_key_from(&mut self, key_data: &[u8]) -> Result<PublicKey, Error> {
        self.round_trip::<PublicKey>(Request::PublicKeyFrom(key_data.into()))
    }
}
