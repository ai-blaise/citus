// FEATURE: T7
// Derived from jackc/pgx pgproto3 (MIT).

//! Authentication-phase message types (tag `R` backend, tag `p` frontend).
//!
//! The pool today proxies `R`/`p` envelopes byte-transparently and does not
//! interpret SCRAM/MD5/GSS exchanges. Typed parsing lives here so future
//! work that needs to inspect or rewrite auth (mTLS handoff, password
//! rotation, SCRAM channel binding) has a single place to plug in.

use crate::codec::{PgReader, PgWriteBuf, WireError};

// --- Backend Authentication (R) -------------------------------------------

/// Sub-codes carried in the first 4 bytes of an `R` frame body.
pub mod auth_codes {
    pub const OK: i32 = 0;
    pub const KERBEROS_V5: i32 = 2;
    pub const CLEARTEXT_PASSWORD: i32 = 3;
    pub const MD5_PASSWORD: i32 = 5;
    pub const SCM_CREDENTIAL: i32 = 6;
    pub const GSS: i32 = 7;
    pub const GSS_CONTINUE: i32 = 8;
    pub const SSPI: i32 = 9;
    pub const SASL: i32 = 10;
    pub const SASL_CONTINUE: i32 = 11;
    pub const SASL_FINAL: i32 = 12;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthenticationRequest {
    Ok,
    KerberosV5,
    CleartextPassword,
    Md5Password { salt: [u8; 4] },
    ScmCredential,
    Gss,
    GssContinue { data: Vec<u8> },
    Sspi,
    Sasl { mechanisms: Vec<String> },
    SaslContinue { data: Vec<u8> },
    SaslFinal { data: Vec<u8> },
}

impl AuthenticationRequest {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'R', |body| match self {
            Self::Ok => body.write_i32(auth_codes::OK),
            Self::KerberosV5 => body.write_i32(auth_codes::KERBEROS_V5),
            Self::CleartextPassword => body.write_i32(auth_codes::CLEARTEXT_PASSWORD),
            Self::Md5Password { salt } => {
                body.write_i32(auth_codes::MD5_PASSWORD);
                body.write_bytes(salt);
            }
            Self::ScmCredential => body.write_i32(auth_codes::SCM_CREDENTIAL),
            Self::Gss => body.write_i32(auth_codes::GSS),
            Self::GssContinue { data } => {
                body.write_i32(auth_codes::GSS_CONTINUE);
                body.write_bytes(data);
            }
            Self::Sspi => body.write_i32(auth_codes::SSPI),
            Self::Sasl { mechanisms } => {
                body.write_i32(auth_codes::SASL);
                for mechanism in mechanisms {
                    body.write_cstring_str(mechanism);
                }
                body.write_u8(0);
            }
            Self::SaslContinue { data } => {
                body.write_i32(auth_codes::SASL_CONTINUE);
                body.write_bytes(data);
            }
            Self::SaslFinal { data } => {
                body.write_i32(auth_codes::SASL_FINAL);
                body.write_bytes(data);
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let code = reader.read_i32()?;
        match code {
            auth_codes::OK => Ok(Self::Ok),
            auth_codes::KERBEROS_V5 => Ok(Self::KerberosV5),
            auth_codes::CLEARTEXT_PASSWORD => Ok(Self::CleartextPassword),
            auth_codes::MD5_PASSWORD => {
                let mut salt = [0u8; 4];
                let bytes = reader.read_slice(4)?;
                salt.copy_from_slice(bytes);
                Ok(Self::Md5Password { salt })
            }
            auth_codes::SCM_CREDENTIAL => Ok(Self::ScmCredential),
            auth_codes::GSS => Ok(Self::Gss),
            auth_codes::GSS_CONTINUE => Ok(Self::GssContinue {
                data: reader.read_to_end().to_vec(),
            }),
            auth_codes::SSPI => Ok(Self::Sspi),
            auth_codes::SASL => {
                let mut mechanisms = Vec::new();
                loop {
                    let next = reader.read_u8()?;
                    if next == 0 {
                        break;
                    }
                    // Continue the cstring starting with `next`.
                    let mut bytes = vec![next];
                    loop {
                        let byte = reader.read_u8()?;
                        if byte == 0 {
                            break;
                        }
                        bytes.push(byte);
                    }
                    let mechanism =
                        String::from_utf8(bytes).map_err(|_| WireError::InvalidUtf8 {
                            field: "sasl-mechanism",
                        })?;
                    mechanisms.push(mechanism);
                }
                Ok(Self::Sasl { mechanisms })
            }
            auth_codes::SASL_CONTINUE => Ok(Self::SaslContinue {
                data: reader.read_to_end().to_vec(),
            }),
            auth_codes::SASL_FINAL => Ok(Self::SaslFinal {
                data: reader.read_to_end().to_vec(),
            }),
            other => Err(WireError::InvalidEnumValue {
                field: "authentication-code",
                value: (other & 0xff) as u8,
            }),
        }
    }
}

// --- Frontend PasswordMessage / SASLInitialResponse / SASLResponse /
// --- GSSResponse (all share tag `p`) --------------------------------------

/// The frontend `p` tag is overloaded: clients send `PasswordMessage` in
/// response to MD5/cleartext, `SASLInitialResponse`/`SASLResponse` for SCRAM,
/// and `GSSResponse` for GSS/SSPI. We model the byte layouts explicitly so
/// the pool can branch on the auth state from the most recent backend `R`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthFrontendFrame {
    Password(PasswordMessageFrame),
    SaslInitialResponse(SaslInitialResponseFrame),
    SaslResponse(SaslResponseFrame),
    GssResponse(GssResponseFrame),
}

impl AuthFrontendFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        match self {
            Self::Password(frame) => frame.encode(buf),
            Self::SaslInitialResponse(frame) => frame.encode(buf),
            Self::SaslResponse(frame) => frame.encode(buf),
            Self::GssResponse(frame) => frame.encode(buf),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PasswordMessageFrame {
    pub password: String,
}

impl PasswordMessageFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'p', |body| {
            body.write_cstring_str(&self.password);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let password = reader.read_cstring_utf8("password-message")?.to_string();
        Ok(Self { password })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SaslInitialResponseFrame {
    pub mechanism: String,
    pub initial_response: Option<Vec<u8>>,
}

impl SaslInitialResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'p', |body| {
            body.write_cstring_str(&self.mechanism);
            match &self.initial_response {
                Some(data) => {
                    body.write_i32(data.len() as i32);
                    body.write_bytes(data);
                }
                None => body.write_i32(-1),
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let mechanism = reader
            .read_cstring_utf8("sasl-initial-mechanism")?
            .to_string();
        let length = reader.read_i32()?;
        let initial_response = if length == -1 {
            None
        } else if length >= 0 {
            Some(reader.read_slice(length as usize)?.to_vec())
        } else {
            return Err(WireError::InvalidLength {
                field: "sasl-initial-response",
                value: length,
            });
        };
        Ok(Self {
            mechanism,
            initial_response,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SaslResponseFrame {
    pub data: Vec<u8>,
}

impl SaslResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'p', |body| {
            body.write_bytes(&self.data);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        Ok(Self {
            data: body.to_vec(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GssResponseFrame {
    pub data: Vec<u8>,
}

impl GssResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'p', |body| {
            body.write_bytes(&self.data);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        Ok(Self {
            data: body.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::read_frame;

    fn roundtrip_request(request: &AuthenticationRequest) -> AuthenticationRequest {
        let mut buf = PgWriteBuf::new();
        request.encode(&mut buf);
        let bytes = buf.into_inner();
        let (_, body, _) = read_frame(&bytes).expect("frame");
        AuthenticationRequest::decode(body).expect("decode")
    }

    #[test]
    fn auth_ok_roundtrip() {
        let original = AuthenticationRequest::Ok;
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_md5_salt_roundtrip() {
        let original = AuthenticationRequest::Md5Password {
            salt: [0xDE, 0xAD, 0xBE, 0xEF],
        };
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_sasl_mechanism_list_roundtrip() {
        let original = AuthenticationRequest::Sasl {
            mechanisms: vec![
                "SCRAM-SHA-256".to_string(),
                "SCRAM-SHA-256-PLUS".to_string(),
            ],
        };
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_sasl_empty_mechanism_list_roundtrip() {
        let original = AuthenticationRequest::Sasl {
            mechanisms: Vec::new(),
        };
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_sasl_single_mechanism_roundtrip() {
        let original = AuthenticationRequest::Sasl {
            mechanisms: vec!["SCRAM-SHA-256".to_string()],
        };
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_sasl_continue_carries_server_data() {
        let original = AuthenticationRequest::SaslContinue {
            data: vec![1, 2, 3, 4, 5],
        };
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_gss_continue_data_roundtrip() {
        let original = AuthenticationRequest::GssContinue {
            data: vec![0xAA; 32],
        };
        assert_eq!(roundtrip_request(&original), original);
    }

    #[test]
    fn auth_invalid_code_rejected() {
        let mut body = Vec::new();
        body.extend_from_slice(&999_i32.to_be_bytes());
        assert!(matches!(
            AuthenticationRequest::decode(&body),
            Err(WireError::InvalidEnumValue { .. })
        ));
    }

    #[test]
    fn password_message_roundtrip() {
        let original = PasswordMessageFrame {
            password: "hunter2".to_string(),
        };
        let mut buf = PgWriteBuf::new();
        original.encode(&mut buf);
        let bytes = buf.into_inner();
        let (header, body, _) = read_frame(&bytes).expect("frame");
        assert_eq!(header.tag, b'p');
        assert_eq!(PasswordMessageFrame::decode(body).unwrap(), original);
    }

    #[test]
    fn sasl_initial_response_with_data_roundtrip() {
        let original = SaslInitialResponseFrame {
            mechanism: "SCRAM-SHA-256".to_string(),
            initial_response: Some(b"n,,n=user,r=nonce".to_vec()),
        };
        let mut buf = PgWriteBuf::new();
        original.encode(&mut buf);
        let bytes = buf.into_inner();
        let (_, body, _) = read_frame(&bytes).expect("frame");
        assert_eq!(SaslInitialResponseFrame::decode(body).unwrap(), original);
    }

    #[test]
    fn sasl_initial_response_with_null_initial_data() {
        let original = SaslInitialResponseFrame {
            mechanism: "SCRAM-SHA-256-PLUS".to_string(),
            initial_response: None,
        };
        let mut buf = PgWriteBuf::new();
        original.encode(&mut buf);
        let bytes = buf.into_inner();
        let (_, body, _) = read_frame(&bytes).expect("frame");
        assert_eq!(SaslInitialResponseFrame::decode(body).unwrap(), original);
    }

    #[test]
    fn sasl_response_data_roundtrip() {
        let original = SaslResponseFrame {
            data: b"client-final-message".to_vec(),
        };
        let mut buf = PgWriteBuf::new();
        original.encode(&mut buf);
        let bytes = buf.into_inner();
        let (_, body, _) = read_frame(&bytes).expect("frame");
        assert_eq!(SaslResponseFrame::decode(body).unwrap(), original);
    }

    #[test]
    fn gss_response_data_roundtrip() {
        let original = GssResponseFrame {
            data: vec![0xBE, 0xEF, 0xFA, 0xCE],
        };
        let mut buf = PgWriteBuf::new();
        original.encode(&mut buf);
        let bytes = buf.into_inner();
        let (_, body, _) = read_frame(&bytes).expect("frame");
        assert_eq!(GssResponseFrame::decode(body).unwrap(), original);
    }
}
