use crate::{command, oath};
use serde::{Deserialize, Serialize};
use trussed::types::ShortData;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Credential {
    pub label: ShortData,
    #[serde(rename = "K")]
    pub kind: oath::Kind,
    #[serde(rename = "A")]
    pub algorithm: oath::Algorithm,
    #[serde(rename = "D")]
    pub digits: u8,
    /// What we get here (inspecting the client app) may not be the raw K, but K' in HMAC lingo,
    /// i.e., If secret.len() < block size (64B for Sha1/Sha256, 128B for Sha512),
    /// then it's the hash of the secret.  Otherwise, it's the secret, padded to length
    /// at least 14B with null bytes. This is of no concern to us, as is it does not
    /// change the MAC.
    ///
    /// The 14 is a bit strange: RFC 4226, section 4 says:
    /// "The algorithm MUST use a strong shared secret.  The length of the shared secret MUST be
    /// at least 128 bits.  This document RECOMMENDs a shared secret length of 160 bits."
    ///
    /// Meanwhile, the client app just pads up to 14B :)

    #[serde(rename = "S")]
    pub secret: ShortData,
    #[serde(rename = "T")]
    pub touch_required: bool,
    #[serde(rename = "C")]
    pub counter: Option<u32>,
}

impl Credential {
    pub fn try_from(credential: &command::Credential) -> Result<Self, ()> {
        Ok(Self {
            label: ShortData::from_slice(credential.label)?,
            kind: credential.kind,
            algorithm: credential.algorithm,
            digits: credential.digits,
            secret: ShortData::from_slice(credential.secret)?,
            touch_required: credential.touch_required,
            counter: credential.counter,
        })
    }
}
