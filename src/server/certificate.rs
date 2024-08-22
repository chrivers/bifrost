use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::str::FromStr;

use camino::Utf8Path;
use der::asn1::{GeneralizedTime, OctetString};
use der::oid::db::rfc4519::COMMON_NAME;
use der::oid::db::rfc5280::ID_KP_SERVER_AUTH;
use der::pem::LineEnding;
use der::{DateTime, EncodePem};
use mac_address::MacAddress;
use p256::ecdsa::DerSignature;
use p256::pkcs8::EncodePrivateKey;
use rand_core::OsRng;
use rsa::pkcs8::SubjectPublicKeyInfoRef;
use sha1::Sha1;
use sha2::Digest;
use x509_cert::attr::AttributeTypeAndValue;
use x509_cert::builder::{Builder, CertificateBuilder, Profile};
use x509_cert::certificate::CertificateInner;
use x509_cert::der::{Decode, Encode};
use x509_cert::ext::pkix::{self, name::GeneralName, ExtendedKeyUsage};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::SubjectPublicKeyInfoOwned;
use x509_cert::time::Validity;
use x509_cert::Certificate;

use crate::error::{ApiError, ApiResult};

#[must_use]
pub fn hue_bridge_id_raw(mac: MacAddress) -> [u8; 8] {
    let b = mac.bytes();
    [b[0], b[1], b[2], 0xFF, 0xFE, b[3], b[4], b[5]]
}

#[must_use]
#[allow(clippy::format_collect)]
pub fn hue_bridge_id(mac: MacAddress) -> String {
    let bytes = hue_bridge_id_raw(mac);
    bytes
        .into_iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// Generate a self-signed X509 certificate, closely matching the type and style
/// used by a real Philips Hue bridge.
///
/// Great care has been taken to match the real certificates as close as
/// possible. These certificates match on the following parameters:
///
///  - Style of subject name
///  - Valid from/to
///  - Type of private key, signing key, and signature type
///  - Choice of X509v3 extensions, including the order of them
///
/// Only these differences are known to remain:
///
///  - This version generates "Extended Key Usage" *with* the "critical"
///    flag set, while the real certificates (unusually) don't.
///    This seems to be a benign difference, and would roughly double
///    the code size needed.
///  - Real hue bridge certificates are signed by a "root-bridge" cert,
///    acting as a kind of CA certificate for the instance certificate.
///    This also seems to have no negative impact.
///
pub fn generate(secret_key: &p256::SecretKey, mac: MacAddress) -> ApiResult<CertificateInner> {
    let public_key = secret_key.public_key();

    let bridge_id = hue_bridge_id(mac);

    let subject = Name::from_str(&format!("CN={bridge_id},O=Philips Hue,C=NL"))?;

    /* self-signed certificate, so subject == issuer */
    let issuer = subject.clone();

    let serial_number = SerialNumber::new(&hue_bridge_id_raw(mac))?;

    /* Philips Hue seems to start their certificates at the beginning of 2017.. */
    let not_before = GeneralizedTime::from_date_time(DateTime::new(2017, 1, 1, 0, 0, 0)?).into();

    /* ..and end on the Y38K boundary (https://en.wikipedia.org/wiki/Year_2038_problem) */
    let not_after = GeneralizedTime::from_date_time(DateTime::new(2038, 1, 19, 3, 14, 7)?).into();

    let validity = Validity {
        not_before,
        not_after,
    };

    /* Use "Manual" profile, since Hue certs have an unusual combination of X509 extensions */
    let profile = Profile::Manual {
        issuer: Some(issuer.clone()),
    };

    let signer = ecdsa::SigningKey::<p256::NistP256>::from(secret_key);
    let pub_key = SubjectPublicKeyInfoOwned::from_key(public_key)?;

    /* Make certificate builder, which will allow us to build the final cert */
    let mut builder = CertificateBuilder::new(
        profile,
        serial_number.clone(),
        validity,
        subject,
        pub_key.clone(),
        &signer,
    )?;

    /* Basic constraints extension */
    builder.add_extension(&pkix::BasicConstraints {
        ca: false,
        path_len_constraint: None,
    })?;

    /* Key Usage extension */
    builder.add_extension(&pkix::KeyUsage(pkix::KeyUsages::DigitalSignature.into()))?;

    let der = pub_key.to_der()?;
    let spki = SubjectPublicKeyInfoRef::from_der(&der)?;

    /* Extended Key Usage extension */
    builder.add_extension(&ExtendedKeyUsage(vec![ID_KP_SERVER_AUTH]))?;

    /* Subject Key Identifier extension */
    builder.add_extension(&pkix::SubjectKeyIdentifier::try_from(spki.clone())?)?;

    /* Authority Key Identifier extension */
    let mut aki = pkix::AuthorityKeyIdentifier::try_from(spki.clone())?;
    aki.key_identifier = Some(OctetString::new(
        Sha1::digest(spki.subject_public_key.raw_bytes()).as_slice(),
    )?);
    aki.authority_cert_issuer = Some(vec![GeneralName::DirectoryName(issuer)]);
    aki.authority_cert_serial_number = Some(serial_number);
    builder.add_extension(&aki)?;

    /* Finally ready to build the certificate */
    Ok(builder.build::<DerSignature>()?)
}

pub fn extract_common_name(rdr: impl Read) -> ApiResult<Option<String>> {
    let bufread = &mut BufReader::new(rdr);

    for chunk in rustls_pemfile::certs(bufread) {
        let cert = Certificate::from_der(&chunk?)?;

        for name in cert.tbs_certificate.subject.0 {
            if let [AttributeTypeAndValue {
                oid: COMMON_NAME,
                value,
            }] = name.0.as_slice()
            {
                return Ok(Some(String::from_utf8(value.value().to_vec())?));
            }
        }
    }

    Ok(None)
}

pub fn generate_and_save(certpath: &Utf8Path, mac: MacAddress) -> ApiResult<()> {
    let secret_key = p256::SecretKey::random(&mut OsRng);
    let cert = generate(&secret_key, mac)?;
    let mut fd = File::create(certpath)?;
    fd.write_all(secret_key.to_pkcs8_pem(LineEnding::LF)?.as_bytes())?;
    fd.write_all(cert.to_pem(LineEnding::LF)?.as_bytes())?;
    Ok(())
}

pub fn check_certificate(certpath: &Utf8Path, mac: MacAddress) -> ApiResult<()> {
    let cn = extract_common_name(File::open(certpath)?)?;
    let id = hue_bridge_id(mac);
    match cn {
        Some(cn) => {
            if cn == id {
                log::debug!("Found existing certificate for bridge id [{id}]");
            } else {
                log::error!("Certificate found, but mac address does not match!");
                log::error!("  [{id}] (expected)");
                log::error!("  [{cn}] {certpath}");
                return Err(ApiError::CertificateInvalid(certpath.to_owned()));
            }
        }
        None => {
            return Err(ApiError::CertificateInvalid(certpath.to_owned()));
        }
    }
    Ok(())
}
