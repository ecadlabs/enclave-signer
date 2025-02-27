use crate::nsm::{self, SharedNSM};
use nitro_signer::{
    aws_config,
    kms_client::{self, ClientFactory},
    rsa, tokio, vsock, Server,
};
use std::io;

pub struct App {
    priv_key: rsa::RsaPrivateKey,
    conf: Config,
    secm: SharedNSM,
}

#[derive(Debug)]
pub enum Error {
    NSM(nsm::Error),
    RSA(rsa::Error),
    IO(io::Error),
}

impl From<nsm::Error> for Error {
    fn from(value: nsm::Error) -> Self {
        Error::NSM(value)
    }
}

impl From<rsa::Error> for Error {
    fn from(value: rsa::Error) -> Self {
        Error::RSA(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IO(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NSM(error) => write!(f, "NSM error: {}", error),
            Error::RSA(error) => write!(f, "RSA error: {}", error),
            Error::IO(error) => write!(f, "IO error: {}", error),
        }
    }
}

impl std::error::Error for Error {}

const RSA_BITS: usize = 2048;
pub const DEFAULT_VSOCK_PORT: u32 = 2000;

#[derive(Debug)]
pub struct Config {
    pub proxy_port: Option<u32>,
    pub proxy_cid: Option<u32>,
    pub endpoint: Option<String>,
    pub listen_port: Option<u32>,
}

impl App {
    pub fn init(conf: Config) -> Result<Self, Error> {
        let secm = nsm::NSM::open()?;
        nsm::seed_rng(&secm, nsm::DEFAULT_ENTROPY_BYTE_SZ)?;

        let mut shared_sm = SharedNSM::new(secm);
        let priv_key = rsa::RsaPrivateKey::new(&mut shared_sm, RSA_BITS)?;

        Ok(Self {
            priv_key,
            conf,
            secm: shared_sm,
        })
    }

    pub async fn run(self) -> Result<(), Error> {
        let client_conf = kms_client::Config {
            proxy_port: self.conf.proxy_port,
            proxy_cid: self.conf.proxy_cid,
            endpoint: self.conf.endpoint,
            client_key: self.priv_key,
        };

        let listen_addr = vsock::SocketAddr::new(
            vsock::VMADDR_CID_ANY,
            self.conf.listen_port.unwrap_or(DEFAULT_VSOCK_PORT),
        );

        let listener = vsock::asio::Listener::bind(&listen_addr)?;
        loop {
            let (conn, addr) = listener.accept().await?;
            println!("incoming connection from {}", addr);

            let ccfg = client_conf.clone();
            let secm = self.secm.clone();

            tokio::spawn(async move {
                let cf = ClientFactory::new(ccfg, aws_config::load_from_env().await, secm.clone());
                let mut srv = Server::new(cf, secm);

                if let Err(err) = srv.serve_connection(conn).await {
                    eprintln!("{}", err);
                }
            });
        }
    }
}
