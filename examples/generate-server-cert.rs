use std::io::{stdout, Write};

use clap::Parser;
use der::{pem::LineEnding, EncodePem};
use mac_address::MacAddress;
use p256::pkcs8::EncodePrivateKey;
use rand_core::OsRng;

use bifrost::{error::ApiResult, server::certificate};

#[derive(Debug, Parser)]
struct Cli {
    mac: MacAddress,
}

fn main() -> ApiResult<()> {
    let args = Cli::parse();

    let secret_key = p256::SecretKey::random(&mut OsRng);
    let cert = certificate::generate(&secret_key, args.mac)?;

    let mut out = stdout().lock();

    out.write_all(secret_key.to_pkcs8_pem(LineEnding::LF)?.as_bytes())?;
    out.write_all(cert.to_pem(LineEnding::LF)?.as_bytes())?;

    Ok(())
}
