// use super::error;
use crate::aes::AesVecBuffer;
use ::aes::cipher;
use ::aes::cipher::generic_array::GenericArray;
use aes::AesCipher;
use aes_gcm_siv::AesGcmSiv;
use aes_gcm_siv::{
    aead::{AeadInPlace, Buffer, KeyInit, OsRng},
    Aes256GcmSiv, Nonce,
};
use std::fmt::Debug;
use std::io::Read;
use std::marker::PhantomData;

pub struct Encrypter<EncryptionProvider> {
    config: EncrypterConfig,
    _provider: PhantomData<EncryptionProvider>,
}

impl<EP> Encrypter<EP> {
    pub fn new(config: EncrypterConfig) -> Self {
        Self {
            _provider: PhantomData,
            config,
        }
    }
}

pub trait Encryptable<EncryptionProvider> {
    fn encrypt(&mut self, input: &str, provider: &mut EncryptionProvider) -> String;
    fn decrypt(&mut self, input: &str) -> String;
}

impl<EncryptionProvider> Encryptable<EncryptionProvider> for Encrypter<EncryptionProvider>
where
    EncryptionProvider: AesEncryptionProviderTrait,
{
    fn encrypt(&mut self, input: &str, provider: &mut EncryptionProvider) -> String {
        let config = &self.config;
        let cipher = &config.cipher;
        let plain_text = input;

        provider.perform_encryption(plain_text, cipher)
    }
    fn decrypt(&mut self, input: &str) -> String {
        "".to_string()
    }
}

pub trait AesEncryptionProviderTrait {
    fn perform_encryption(&mut self, plain_text: &str, cipher: &AesCipher) -> String;
}

pub struct AesEncryptionProvide<'a> {
    pub buffer: crate::aes::AesVecBuffer<'a, ()>,
}

impl<'a> AesEncryptionProvide<'a> {
    fn new() -> Self {
        Self {
            buffer: AesVecBuffer::<()>::new(),
        }
    }

    /// Hex encoded ciphertext
    fn ciphertext_hex(&mut self) -> String {
        let text = hex::encode(self.buffer.inner().to_vec());

        text
    }
}

impl<'a> AesEncryptionProviderTrait for AesEncryptionProvide<'a> {
    fn perform_encryption(&mut self, plain_text: &str, cipher: &AesCipher) -> String {
        let (cipher, nonce) = (&cipher.cipher, &cipher.nonce);

        // Note: buffer needs 16-bytes overhead for auth tag tag
        self.buffer
            .extend_from_slice(plain_text.as_bytes())
            .unwrap();

        cipher
            .encrypt_in_place(nonce, b"", &mut self.buffer)
            .map_err(|err| -> crate::error::Result<()> {
                let err = format!(
                    "[{}] Failed to encrypt due to {}.",
                    env!("CARGO_CRATE_NAME"),
                    err.to_string(),
                );
                Err(crate::DefaultError::ErrorMessage(err))
            })
            .expect("Encrypt cipher in place");

        self.ciphertext_hex()
    }
}

#[cfg(test)]
mod encryptable {
    use super::Encryptable;
    use super::EncrypterConfig;
    use crate::encrypter::AesEncryptionProvide;
    use crate::hasher::Hashable;

    #[test]
    fn test_encrypter() {
        const PBKDF_ROUNDS: u32 = 2;
        let buf = [0u8; crate::hasher::KEY_BUFF_SIZE];
        let mut buf_boxed = Box::new(buf);

        let hasher =
            &mut crate::hasher::HashProvider::<crate::hasher::PrfHasher>::new(&mut buf_boxed);
        let pbkdf_key = hasher
            .pbkdf2_gen("password", "salt", &PBKDF_ROUNDS)
            .unwrap();
        let pbkdf_key_hex = hex::encode(pbkdf_key);

        let config = EncrypterConfig::new(pbkdf_key_hex);

        // Create Encrypter
        let mut provider = AesEncryptionProvide::new();
        let mut enc = super::Encrypter::<AesEncryptionProvide>::new(config);
        let r = enc.encrypt("secret nuke codes", &mut provider);

        assert_ne!(r, "")
    }
}

pub struct EncrypterConfig {
    pub hash_key: String,
    pub cipher: AesCipher,
}

impl EncrypterConfig {
    pub fn new(hash_key: String) -> Self {
        let key = Aes256GcmSiv::generate_key(&mut OsRng);
        let cipher = Aes256GcmSiv::new(&key);

        // Generate nonce
        let mut bytes = hash_key.as_bytes();
        let mut short_nonce = [0u8; 12];
        bytes
            .read_exact(&mut short_nonce)
            .expect("Nonce is too short");
        let nonce: &GenericArray<u8, cipher::consts::U12> = Nonce::from_slice(&short_nonce[..]); // 96-bits; unique per message

        let cipher = AesCipher {
            cipher,
            nonce: *nonce,
        };

        Self { hash_key, cipher }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hasher::Hashable;

    #[test]
    fn create_aes_config() {
        const PBKDF_ROUNDS: u32 = 2;
        let buf = [0u8; crate::hasher::KEY_BUFF_SIZE];
        let mut buf_boxed = Box::new(buf);

        let hasher =
            &mut crate::hasher::HashProvider::<crate::hasher::PrfHasher>::new(&mut buf_boxed);
        let pbkdf_key = hasher
            .pbkdf2_gen("password", "salt", &PBKDF_ROUNDS)
            .unwrap();
        let pbkdf_key_hex = hex::encode(pbkdf_key);

        let _config = EncrypterConfig::new(pbkdf_key_hex);
    }
}

pub mod aes {
    use super::*;

    pub struct AesCipher {
        pub cipher: AesGcmSiv<::aes::Aes256>,
        pub nonce: GenericArray<u8, cipher::consts::U12>,
    }
}
