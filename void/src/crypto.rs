/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::store::Error;
use aes_gcm::{
    aead::{Aead, KeyInit, consts::U16},
    aes::Aes256,
    AesGcm, Nonce,
};
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use hkdf::Hkdf;
use sha2::Sha256;
use std::result::Result;
use std::vec::Vec;
use uuid::Uuid;

/// Returns a [u8; 32] array with the value of the hash.
/// It uses Blake2B as hasher.
///
/// # Arguments
///
/// * `data` - A byte array that holds the data to be hashed.
/// * `salt` - A byte array that holds the salt. [u0; 16].
///
/// # Example
///
/// ```ignore
/// let name = "Álan Crístoffer";
/// let salt = uuid();
/// let hash = hash(name.as_bytes(), &salt);
/// ```
pub(crate) fn hash(data: &[u8], salt: &[u8; 16]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut hasher = Blake2bVar::new(32).unwrap();
    hasher.update(data);
    hasher.update(b"$");
    hasher.update(salt);
    hasher.finalize_variable(&mut hash).expect("Error hashing");
    hash
}

/// Returns a [u8; 16] array with the value of the UUID.
/// It uses UUIDv4.
///
/// # Example
///
/// ```ignore
/// let uuid = uuid();
/// ```
pub(crate) fn uuid() -> [u8; 16] {
    *Uuid::new_v4().as_bytes()
}

/// Returns a [u8; 32] array with the value of the key.
/// It uses HKDF for key derivation.
///
/// A key derivator algorithm will generate a cryptographic key from a
/// password, salt and IV. It does so in a way that is inefficient to run
/// many times in a row, preventing repetition attacks. It is also the best
/// way to store passwords in databases.
///
/// # Arguments
///
/// * `pswd` - A string that holds the password.
/// * `salt` - A byte array that holds the salt. [u0; 16].
/// * `iv` - A byte array that holds the initial value. [u0; 16].
///
/// # Example
///
/// ```ignore
/// let pswd = "123456";
/// let salt = uuid();
/// let iv = uuid();
/// let dkey = derive_key(&pswd, &salt, &iv);
/// ```
pub(crate) fn derive_key(pswd: &str, salt: &[u8; 16], iv: &[u8; 16]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), iv);
    let mut key = [0u8; 32];
    hk.expand(pswd.as_bytes(), &mut key)
        .expect("Error generating key.");
    key
}

/// Encrypts data using AES 256 and returns bytes as a Vec<u8>.
/// If key or iv are larger than needed, it will be truncated.
///
/// # Arguments
///
/// * `data` - Data to be encrypted.
/// * `key` - A byte array that holds the key. [u0; 32].
/// * `iv` - A byte array that holds the initial value. [u0; 16].
///
/// # Example
///
/// ```ignore
/// let pswd = "123456";
/// let salt = hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
/// let iv = hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
/// let dkey = derive_key(&pswd, &salt, &iv);
/// let msg = "Hello World!";
/// let cipher = encrypt(msg.as_bytes(), &dkey, &iv);
/// ```
pub(crate) fn encrypt(data: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Result<Vec<u8>, Error> {
    let aes = AesGcm::<Aes256, U16>::new(key.into());
    let nonce = Nonce::from_slice(iv);
    aes.encrypt(nonce, data)
        .map_err(|_| Error::CannotEncryptFileError)
}

/// Decrypts data using AES 256 and returns bytes as a Vec<u8> in a Result.
/// If key or iv are larger than needed, it will be truncated.
///
/// # Arguments
///
/// * `data` - Data to be decrypted.
/// * `key` - A byte array that holds the key. [u0; 32].
/// * `iv` - A byte array that holds the initial value. [u0; 16].
///
/// # Example
///
/// ```ignore
/// let pswd = "123456";
/// let salt = hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
/// let iv = hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
/// let dkey = derive_key(&pswd, &salt, &iv);
/// let msg = hex::decode("d272fba8e0b673060cdf3666e7a0913e").unwrap();
/// let cipher = decrypt(&msg, &dkey, &iv).unwrap();
/// ```
pub(crate) fn decrypt(data: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Result<Vec<u8>, Error> {
    let aes = AesGcm::<Aes256, U16>::new(key.into());
    let nonce = Nonce::from_slice(iv);
    aes.decrypt(nonce, data)
        .map_err(|_| Error::CannotDecryptFileError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid() {
        let uuid = uuid();
        assert!(uuid.len() > 0);
    }

    #[test]
    fn test_hash() {
        let name = "Álan Crístoffer";
        let salt_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let mut salt = [0u8; 16];
        salt.copy_from_slice(salt_vec.as_slice());
        let hash = hash(name.as_bytes(), &salt);
        assert_eq!(
            "f5467b71433f075b7731aa77e5bb6d94165cedf2f45c7854b7be9283bb6dc404",
            hex::encode(&hash)
        );
    }

    #[test]
    fn test_derive_key() {
        let pswd = "123456";
        let salt_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let iv_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let mut salt = [0u8; 16];
        let mut iv = [0u8; 16];
        salt.copy_from_slice(salt_vec.as_slice());
        iv.copy_from_slice(iv_vec.as_slice());
        let dkey = derive_key(&pswd, &salt, &iv);
        assert_eq!(
            "aa4458163f34dcd687a600beba020c1f3c9351b18b38b8cc981a86be69b4cee4",
            hex::encode(&dkey)
        );
    }

    #[test]
    fn test_encrypt() {
        let pswd = "123456";
        let salt_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let iv_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let mut salt = [0u8; 16];
        let mut iv = [0u8; 16];
        salt.copy_from_slice(salt_vec.as_slice());
        iv.copy_from_slice(iv_vec.as_slice());
        let dkey = derive_key(&pswd, &salt, &iv);
        let msg = "Hello World!";
        let cipher = encrypt(msg.as_bytes(), &dkey, &iv).expect("Error encrypting");
        assert_eq!("74536b5f588078d9c70363a4c7b35deea4f2902a8bed6f693bfeffba", hex::encode(&cipher));
    }

    #[test]
    fn test_decrypt() {
        let pswd = "123456";
        let salt_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let iv_vec =
            hex::decode("8a5eaba62bf74487ac35ce27050445cd").expect("Could not decode salt");
        let mut salt = [0u8; 16];
        let mut iv = [0u8; 16];
        salt.copy_from_slice(salt_vec.as_slice());
        iv.copy_from_slice(iv_vec.as_slice());
        let dkey = derive_key(&pswd, &salt, &iv);
        let msg = hex::decode("74536b5f588078d9c70363a4c7b35deea4f2902a8bed6f693bfeffba").unwrap();
        let cipher = decrypt(&msg, &dkey, &iv).unwrap();
        assert_eq!("Hello World!", std::str::from_utf8(&cipher).unwrap());
    }
}
