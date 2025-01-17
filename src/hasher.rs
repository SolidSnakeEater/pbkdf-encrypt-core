use crate::error;
use hmac::{digest::core_api::CoreWrapper, EagerHash, Hmac, HmacCore};
use pbkdf2::pbkdf2;
use sha2::Sha512;
use std::{fmt::Debug, marker::PhantomData};

pub type PrfHasher = Sha512;
pub const KEY_BUFF_SIZE: usize = 20;

pub trait Hashable<H> {
    type KeyBuf;

    fn pbkdf2_gen(
        &mut self,
        password: &str,
        salt: &str,
        rounds: &u32,
    ) -> error::Result<Self::KeyBuf>;
}

#[derive(Debug)]
pub struct HashProvider<'a, H> {
    _hasher: PhantomData<H>,
    key: &'a mut Box<[u8; KEY_BUFF_SIZE]>,
}

impl<'a, H> HashProvider<'a, H> {
    pub fn new(buf: &'a mut Box<[u8; KEY_BUFF_SIZE]>) -> Self {
        Self {
            _hasher: PhantomData,
            key: buf,
        }
    }
}

impl<'a, H> Hashable<H> for HashProvider<'a, H>
where
    CoreWrapper<HmacCore<H>>: hmac::KeyInit,
    H: hmac::EagerHash,
    <H as EagerHash>::Core: Sync,
{
    type KeyBuf = [u8; KEY_BUFF_SIZE];

    fn pbkdf2_gen(
        &mut self,
        password: &str,
        salt: &str,
        rounds: &u32,
    ) -> error::Result<Self::KeyBuf>
where {
        pbkdf2::<Hmac<H>>(
            &password.to_string().as_bytes(),
            &salt.to_string().as_bytes(),
            *rounds,
            self.key.as_mut(),
            // fmt
        )
        .expect("HMAC can be initialized with any key length");

        Ok(*self.key.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::Hashable;
    use hex_literal::hex;

    #[test]
    fn generate_pbkdf2_key() {
        const PBKDF_ROUNDS: u32 = 2;
        let buf = [0u8; crate::hasher::KEY_BUFF_SIZE];
        let mut buf_boxed = Box::new(buf);

        let hasher =
            &mut crate::hasher::HashProvider::<crate::hasher::PrfHasher>::new(&mut buf_boxed);
        let pbkdf_key = hasher
            .pbkdf2_gen("password", "salt", &PBKDF_ROUNDS)
            .unwrap();

        // NOTE: Compute hex string for the number of rounds provided above; this affects the pbkdf key
        // and the test will fail if the number of rounds are changed.
        // let pbkdf_key_hex = hex::encode(pbkdf_key);

        assert_eq!(
            &pbkdf_key,
            &hex!("e1d9c16aa681708a45f5c7c4e215ceb66e011a2e")
        );
    }
}
