/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2020 Álan Crístoffer e Sousa
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, lish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use aes::Aes256;
use blake2::digest::{Update, VariableOutput};
use blake2::{Blake2b, VarBlake2b};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, BlockModeError, Cbc};
use hkdf::Hkdf;
use std::result::Result;
use std::vec::Vec;
use uuid::Uuid;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

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
    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(data);
    hasher.update(b"$");
    hasher.update(salt);
    hasher.finalize_variable(|res| hash.copy_from_slice(&res));
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
    Uuid::new_v4().as_bytes().clone()
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
    let hk = Hkdf::<Blake2b>::new(Some(salt), iv);
    let mut key = [0u8; 32];
    hk.expand(pswd.as_bytes(), &mut key)
        .expect("Error generating key.");
    key
}

///
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
pub(crate) fn encrypt(data: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes256Cbc::new_var(key, iv).unwrap();
    cipher.encrypt_vec(data)
}

///
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
pub(crate) fn decrypt(
    data: &[u8],
    key: &[u8; 32],
    iv: &[u8; 16],
) -> Result<Vec<u8>, BlockModeError> {
    let cipher = Aes256Cbc::new_var(key, iv).unwrap();
    cipher.decrypt_vec(&data)
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
            "b5e2214aed79c71a5303364097c165e256f3bbf9be234dbc7b23ac82aa5d2561",
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
        let cipher = encrypt(msg.as_bytes(), &dkey, &iv);
        assert_eq!("d272fba8e0b673060cdf3666e7a0913e", hex::encode(&cipher));
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
        let msg = hex::decode("d272fba8e0b673060cdf3666e7a0913e").unwrap();
        let cipher = decrypt(&msg, &dkey, &iv).unwrap();
        assert_eq!("Hello World!", std::str::from_utf8(&cipher).unwrap());
    }
}
