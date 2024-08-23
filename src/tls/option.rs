//! Tls Option

use std::{
    fs,
    io::{BufReader, Cursor},
    path::PathBuf,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, PrivateKeyDer},
    ClientConfig, ServerConfig, SignatureScheme,
};

use super::TlsError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct TlsClientOption {
    pub insecure: bool,
    pub alpn: Vec<String>,
    pub enable_sni: bool,
    pub server_name: String,
}

impl Default for TlsClientOption {
    fn default() -> Self {
        Self {
            insecure: false,
            alpn: vec![],
            enable_sni: true,
            server_name: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TlsServerOption {
    #[serde(default)]
    pub alpn: Vec<String>,
    pub certificate: TlsCertOption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TlsCertOption {
    File { cert: PathBuf, key: PathBuf },
    Text { certs: Vec<String>, key: String },
}

impl TryFrom<TlsClientOption> for rustls::ClientConfig {
    type Error = TlsError;

    fn try_from(opt: TlsClientOption) -> Result<Self, Self::Error> {
        let mut config = if opt.insecure {
            ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoServerCertVerifier))
                .with_no_client_auth()
        } else {
            let root_store = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.iter().cloned().collect(),
            };
            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        config.enable_sni = opt.enable_sni;

        if !opt.alpn.is_empty() {
            config.alpn_protocols = opt
                .alpn
                .into_iter()
                .map(|s| s.into_bytes())
                .collect::<Vec<_>>();
        }

        Ok(config)
    }
}

impl TryFrom<TlsServerOption> for ServerConfig {
    type Error = TlsError;

    fn try_from(option: TlsServerOption) -> Result<Self, Self::Error> {
        let (certs, key) = match option.certificate {
            TlsCertOption::File { cert, key } => {
                let mut cert_reader = BufReader::new(fs::File::open(&cert)?);
                let mut key_reader = BufReader::new(fs::File::open(&key)?);

                (
                    load_certs(&mut cert_reader)?,
                    load_priv_key(&mut key_reader)?,
                )
            }
            TlsCertOption::Text { certs, key } => {
                let mut cert_reader = BufReader::new(Cursor::new(certs.join("\n")));
                let mut key_reader = BufReader::new(Cursor::new(key));

                (
                    load_certs(&mut cert_reader)?,
                    load_priv_key(&mut key_reader)?,
                )
            }
        };

        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| TlsError::InvalidCert(e.to_string()))?;

        if !option.alpn.is_empty() {
            config.alpn_protocols = option
                .alpn
                .into_iter()
                .map(|s| s.into_bytes())
                .collect::<Vec<_>>();
        }

        Ok(config)
    }
}

pub fn load_certs<R: std::io::Read>(
    reader: &mut BufReader<R>,
) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let certs = rustls_pemfile::certs(reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| TlsError::InvalidCert(e.to_string()))?;

    Ok(certs)
}

pub fn load_priv_key<R: std::io::Read>(
    reader: &mut BufReader<R>,
) -> Result<PrivateKeyDer<'static>, TlsError> {
    let key = rustls_pemfile::private_key(reader)
        .map_err(|e| TlsError::InvalidKey(e.to_string()))?
        .ok_or(TlsError::InvalidKey("not found".to_string()))?;

    Ok(key)
}

#[derive(Debug)]
struct NoServerCertVerifier;

impl ServerCertVerifier for NoServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}
