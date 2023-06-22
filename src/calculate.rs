// Copyright (C) 2021-2022 The Trussed Developers
// Copyright (C) 2023 Nitrokey GmbH
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::convert::TryInto;
use iso7816::Status;

use crate::oath;
use crate::Result;
use trussed::types::Signature;
use trussed::{
    client, try_syscall,
    types::{KeyId, Location},
};

fn with_key<T, F, O>(trussed: &mut T, key: &[u8], f: F) -> Result<O>
where
    T: client::Client,
    F: FnOnce(&mut T, KeyId) -> O,
{
    let injected = try_syscall!(trussed.unsafe_inject_shared_key(key, Location::Volatile, trussed::key::Kind::Shared(key.len())))
        .map_err(|_| Status::UnspecifiedNonpersistentExecutionError)?
        .key;
    let res = f(trussed, injected);
    try_syscall!(trussed.delete(injected)).ok();
    Ok(res)
}
/// The core calculation
///
/// [RFC 4226][rfc-4226] (HOTP) only defines HMAC-SHA1
/// [RFC 6238][rfc-6238] (TOTP) also allows use of HMAC-SHA256 and HMAC-SHA512
///
/// [rfc-4226]: https://tools.ietf.org/html/rfc4226
/// [rfc-6238]: https://tools.ietf.org/html/rfc6238
pub fn calculate<T>(
    trussed: &mut T,
    algorithm: oath::Algorithm,
    challenge: &[u8],
    key: &[u8],
) -> Result<[u8; 4]>
where
    T: client::Client + client::HmacSha1 + client::HmacSha256 + client::Sha256,
{
    with_key(trussed, key, |trussed, key| {
        use oath::Algorithm::*;
        let truncated = match algorithm {
            Sha1 => {
                let digest = try_syscall!(trussed.sign_hmacsha1(key, challenge))
                    .map_err(|_| Status::UnspecifiedPersistentExecutionError)?
                    .signature;
                dynamic_truncation(&digest)
            }
            Sha256 => {
                let digest = try_syscall!(trussed.sign_hmacsha256(key, challenge))
                    .map_err(|_| Status::UnspecifiedPersistentExecutionError)?
                    .signature;
                dynamic_truncation(&digest)
            }
            Sha512 => return Err(Status::FunctionNotSupported),
        };

        Ok(truncated.to_be_bytes())
    })?
}

pub fn hmac_challenge<T>(
    trussed: &mut T,
    algorithm: oath::Algorithm,
    challenge: &[u8],
    key: &[u8],
) -> Result<Signature>
where
    T: client::Client + client::HmacSha1,
{
    with_key(trussed, key, |trussed, key| {
        use oath::Algorithm::*;
        match algorithm {
            Sha1 => {
                let digest = try_syscall!(trussed.sign_hmacsha1(key, challenge))
                    .map_err(|_| Status::UnspecifiedPersistentExecutionError)?
                    .signature;
                Ok(digest)
            }
            _ => Err(Status::InstructionNotSupportedOrInvalid),
        }
    })?
}

fn dynamic_truncation(digest: &[u8]) -> u32 {
    // TL;DR: The standard assumes that you use the low 4 bits of the last byte of the hash, regardless of its length. So replace 19 in the original DT definition with 31 for SHA-256 or 63 for SHA-512 and you are good to go.

    // low-order bits of last byte
    let offset_bits = (*digest.last().unwrap() & 0xf) as usize;

    //
    let p = u32::from_be_bytes(digest[offset_bits..][..4].try_into().unwrap());

    // zero highest bit, avoids signed/unsigned "ambiguity"
    p & 0x7fff_ffff
}

// fn hmac_and_truncate(key: &[u8], message: &[u8], digits: u32) -> u64 {
//     use hmac::{Hmac, Mac, NewMac};
//     // let mut hmac = Hmac::<D>::new(GenericArray::from_slice(key));
//     let mut hmac = Hmac::<sha1::Sha1>::new_varkey(key).unwrap();
//     hmac.update(message);
//     let result = hmac.finalize();

//     // output of `.code()` is GenericArray<u8, OutputSize>, again 20B
//     // crypto-mac docs warn: "Be very careful using this method,
//     // since incorrect use of the code material may permit timing attacks
//     // which defeat the security provided by the Mac trait."
//     let hs = result.into_bytes();

//     dynamic_truncation(&hs) % 10_u64.pow(digits)
// }
