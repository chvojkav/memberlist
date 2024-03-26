use std::{iter::once, sync::Arc};

use async_lock::RwLock;
use indexmap::IndexSet;

/// The key used while attempting to encrypt/decrypt a message
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecretKey {
  /// secret key for AES128
  Aes128([u8; 16]),
  /// secret key for AES192
  Aes192([u8; 24]),
  /// secret key for AES256
  Aes256([u8; 32]),
}

#[cfg(feature = "serde")]
const _: () = {
  use base64::Engine;
  use serde::{Deserialize, Serialize};

  impl Serialize for SecretKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::Serializer,
    {
      if serializer.is_human_readable() {
        base64::engine::general_purpose::STANDARD
          .encode(self)
          .serialize(serializer)
      } else {
        serializer.serialize_bytes(self)
      }
    }
  }

  impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: serde::Deserializer<'de>,
    {
      macro_rules! parse {
        ($key:ident) => {{
          match $key.len() {
            16 => Ok(Self::Aes128($key.try_into().unwrap())),
            24 => Ok(Self::Aes192($key.try_into().unwrap())),
            32 => Ok(Self::Aes256($key.try_into().unwrap())),
            _ => Err(<D::Error as serde::de::Error>::custom(
              "invalid secret key length",
            )),
          }
        }};
      }

      if deserializer.is_human_readable() {
        <String as Deserialize<'de>>::deserialize(deserializer).and_then(|val| {
          base64::engine::general_purpose::STANDARD
            .decode(val)
            .map_err(serde::de::Error::custom)
            .and_then(|key| parse!(key))
        })
      } else {
        <Vec<u8> as Deserialize<'de>>::deserialize(deserializer).and_then(|val| parse!(val))
      }
    }
  }
};

impl core::hash::Hash for SecretKey {
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    self.as_ref().hash(state);
  }
}

impl core::borrow::Borrow<[u8]> for SecretKey {
  fn borrow(&self) -> &[u8] {
    self.as_ref()
  }
}

impl PartialEq<[u8]> for SecretKey {
  fn eq(&self, other: &[u8]) -> bool {
    self.as_ref() == other
  }
}

impl core::ops::Deref for SecretKey {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    match self {
      Self::Aes128(k) => k,
      Self::Aes192(k) => k,
      Self::Aes256(k) => k,
    }
  }
}

impl core::ops::DerefMut for SecretKey {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      Self::Aes128(k) => k,
      Self::Aes192(k) => k,
      Self::Aes256(k) => k,
    }
  }
}

impl From<[u8; 16]> for SecretKey {
  fn from(k: [u8; 16]) -> Self {
    Self::Aes128(k)
  }
}

impl From<[u8; 24]> for SecretKey {
  fn from(k: [u8; 24]) -> Self {
    Self::Aes192(k)
  }
}

impl From<[u8; 32]> for SecretKey {
  fn from(k: [u8; 32]) -> Self {
    Self::Aes256(k)
  }
}

impl TryFrom<&[u8]> for SecretKey {
  type Error = String;

  fn try_from(k: &[u8]) -> Result<Self, Self::Error> {
    match k.len() {
      16 => Ok(Self::Aes128(k.try_into().unwrap())),
      24 => Ok(Self::Aes192(k.try_into().unwrap())),
      32 => Ok(Self::Aes256(k.try_into().unwrap())),
      x => Err(format!(
        "invalid key size: {}, secret key size must be 16, 24 or 32 bytes",
        x
      )),
    }
  }
}

impl AsRef<[u8]> for SecretKey {
  fn as_ref(&self) -> &[u8] {
    match self {
      Self::Aes128(k) => k,
      Self::Aes192(k) => k,
      Self::Aes256(k) => k,
    }
  }
}

impl AsMut<[u8]> for SecretKey {
  fn as_mut(&mut self) -> &mut [u8] {
    match self {
      Self::Aes128(k) => k,
      Self::Aes192(k) => k,
      Self::Aes256(k) => k,
    }
  }
}

smallvec_wrapper::smallvec_wrapper!(
  /// A collection of secret keys, you can just treat it as a `Vec<SecretKey>`.
  #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
  #[repr(transparent)]
  #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
  #[cfg_attr(feature = "serde", serde(transparent))]
  pub SecretKeys([SecretKey; 3]);
);

/// Error for [`SecretKeyring`]
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SecretKeyringError {
  /// Secret key is not in the keyring
  #[error("secret key is not in the keyring")]
  SecretKeyNotFound,
  /// Removing the primary key is not allowed
  #[error("removing the primary key is not allowed")]
  RemovePrimaryKey,
}

#[derive(Debug)]
pub(super) struct SecretKeyringInner {
  pub(super) primary_key: SecretKey,
  pub(super) keys: IndexSet<SecretKey>,
}

/// A lock-free and thread-safe container for a set of encryption keys.
/// The keyring contains all key data used internally by memberlist.
///
/// If creating a keyring with multiple keys, one key must be designated
/// primary by passing it as the primaryKey. If the primaryKey does not exist in
/// the list of secondary keys, it will be automatically added at position 0.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct SecretKeyring {
  pub(super) inner: Arc<RwLock<SecretKeyringInner>>,
}

impl SecretKeyring {
  /// Constructs a new container for a primary key. The
  /// keyring contains all key data used internally by memberlist.
  ///
  /// If only a primary key is passed, then it will be automatically added to the
  /// keyring.
  ///
  /// A key should be either 16, 24, or 32 bytes to select AES-128,
  /// AES-192, or AES-256.
  #[inline]
  pub fn new(primary_key: SecretKey) -> Self {
    Self {
      inner: Arc::new(RwLock::new(SecretKeyringInner {
        primary_key,
        keys: IndexSet::new(),
      })),
    }
  }

  /// Constructs a new container for a set of encryption keys. The
  /// keyring contains all key data used internally by memberlist.
  ///
  /// If only a primary key is passed, then it will be automatically added to the
  /// keyring. If creating a keyring with multiple keys, one key must be designated
  /// primary by passing it as the primaryKey. If the primaryKey does not exist in
  /// the list of secondary keys, it will be automatically added.
  ///
  /// A key should be either 16, 24, or 32 bytes to select AES-128,
  /// AES-192, or AES-256.
  #[inline]
  pub fn with_keys(
    primary_key: SecretKey,
    keys: impl Iterator<Item = impl Into<SecretKey>>,
  ) -> Self {
    if keys.size_hint().0 != 0 {
      return Self {
        inner: Arc::new(RwLock::new(SecretKeyringInner {
          primary_key,
          keys: keys
            .filter_map(|k| {
              let k = k.into();
              if k == primary_key {
                None
              } else {
                Some(k)
              }
            })
            .collect(),
        })),
      };
    }

    Self::new(primary_key)
  }

  /// Returns the key on the ring at position 0. This is the key used
  /// for encrypting messages, and is the first key tried for decrypting messages.
  #[inline]
  pub async fn primary_key(&self) -> SecretKey {
    self.inner.read().await.primary_key
  }

  /// Drops a key from the keyring. This will return an error if the key
  /// requested for removal is currently at position 0 (primary key).
  #[inline]
  pub async fn remove(&self, key: &[u8]) -> Result<(), SecretKeyringError> {
    let mut inner = self.inner.write().await;
    if &inner.primary_key == key {
      return Err(SecretKeyringError::RemovePrimaryKey);
    }
    inner.keys.shift_remove(key);
    Ok(())
  }

  /// Install a new key on the ring. Adding a key to the ring will make
  /// it available for use in decryption. If the key already exists on the ring,
  /// this function will just return noop.
  ///
  /// key should be either 16, 24, or 32 bytes to select AES-128,
  /// AES-192, or AES-256.
  #[inline]
  pub async fn insert(&self, key: SecretKey) {
    self.inner.write().await.keys.insert(key);
  }

  /// Changes the key used to encrypt messages. This is the only key used to
  /// encrypt messages, so peers should know this key before this method is called.
  #[inline]
  pub async fn use_key(&self, key_data: &[u8]) -> Result<(), SecretKeyringError> {
    let mut inner = self.inner.write().await;
    if key_data == inner.primary_key.as_ref() {
      return Ok(());
    }

    // Try to find the key to set as primary
    let Some(&key) = inner.keys.get(key_data) else {
      return Err(SecretKeyringError::SecretKeyNotFound);
    };

    let old_pk = inner.primary_key;
    inner.keys.insert(old_pk);
    inner.primary_key = key;
    inner.keys.swap_remove(key_data);
    Ok(())
  }

  /// Returns the current set of keys on the ring.
  #[inline]
  pub async fn keys(&self) -> impl Iterator<Item = SecretKey> + 'static {
    let inner = self.inner.read().await;

    // we must promise the first key is the primary key
    // so that when decrypt messages, we can try the primary key first
    once(inner.primary_key).chain(inner.keys.clone().into_iter())
  }
}

#[cfg(test)]
mod tests {
  use std::ops::{Deref, DerefMut};

  use super::*;

  #[test]
  fn test_secret_key() {
    let mut key = SecretKey::from([0; 16]);
    assert_eq!(key.deref(), &[0; 16]);
    assert_eq!(key.deref_mut(), &mut [0; 16]);
    assert_eq!(key.as_ref(), &[0; 16]);
    assert_eq!(key.as_mut(), &mut [0; 16]);
    assert_eq!(key.len(), 16);
    assert!(!key.is_empty());
    assert_eq!(key.to_vec(), vec![0; 16]);

    let mut key = SecretKey::from([0; 24]);
    assert_eq!(key.deref(), &[0; 24]);
    assert_eq!(key.deref_mut(), &mut [0; 24]);
    assert_eq!(key.as_ref(), &[0; 24]);
    assert_eq!(key.as_mut(), &mut [0; 24]);
    assert_eq!(key.len(), 24);
    assert!(!key.is_empty());
    assert_eq!(key.to_vec(), vec![0; 24]);

    let mut key = SecretKey::from([0; 32]);
    assert_eq!(key.deref(), &[0; 32]);
    assert_eq!(key.deref_mut(), &mut [0; 32]);
    assert_eq!(key.as_ref(), &[0; 32]);
    assert_eq!(key.as_mut(), &mut [0; 32]);
    assert_eq!(key.len(), 32);
    assert!(!key.is_empty());
    assert_eq!(key.to_vec(), vec![0; 32]);

    let mut key = SecretKey::from([0; 16]);
    assert_eq!(key.as_ref(), &[0; 16]);
    assert_eq!(key.as_mut(), &mut [0; 16]);

    let mut key = SecretKey::from([0; 24]);
    assert_eq!(key.as_ref(), &[0; 24]);
    assert_eq!(key.as_mut(), &mut [0; 24]);

    let mut key = SecretKey::from([0; 32]);
    assert_eq!(key.as_ref(), &[0; 32]);
    assert_eq!(key.as_mut(), &mut [0; 32]);

    let key = SecretKey::Aes128([0; 16]);
    assert_eq!(key.to_vec(), vec![0; 16]);

    let key = SecretKey::Aes192([0; 24]);
    assert_eq!(key.to_vec(), vec![0; 24]);

    let key = SecretKey::Aes256([0; 32]);
    assert_eq!(key.to_vec(), vec![0; 32]);
  }

  #[test]
  fn test_try_from() {
    assert!(SecretKey::try_from([0; 15].as_slice()).is_err());
    assert!(SecretKey::try_from([0; 16].as_slice()).is_ok());
    assert!(SecretKey::try_from([0; 23].as_slice()).is_err());
    assert!(SecretKey::try_from([0; 24].as_slice()).is_ok());
    assert!(SecretKey::try_from([0; 31].as_slice()).is_err());
    assert!(SecretKey::try_from([0; 32].as_slice()).is_ok());
  }

  const TEST_KEYS: &[SecretKey] = &[
    SecretKey::Aes128([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
    SecretKey::Aes128([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]),
    SecretKey::Aes128([8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7]),
  ];

  #[tokio::test]
  async fn test_primary_only() {
    let keyring = SecretKeyring::new(TEST_KEYS[1]);
    assert_eq!(keyring.keys().await.collect::<Vec<_>>().len(), 1);
  }

  #[tokio::test]
  async fn test_get_primary_key() {
    let keyring = SecretKeyring::with_keys(TEST_KEYS[1], TEST_KEYS.iter().copied());
    assert_eq!(keyring.primary_key().await.as_ref(), TEST_KEYS[1].as_ref());
  }

  #[tokio::test]
  async fn test_insert_remove_use() {
    let keyring = SecretKeyring::new(TEST_KEYS[1]);

    // Use non-existent key throws error
    keyring.use_key(&TEST_KEYS[2]).await.unwrap_err();

    // Add key to ring
    keyring.insert(TEST_KEYS[2]).await;
    assert_eq!(keyring.inner.read().await.keys.len() + 1, 2);
    assert_eq!(keyring.keys().await.next().unwrap(), TEST_KEYS[1]);

    // Use key that exists should succeed
    keyring.use_key(&TEST_KEYS[2]).await.unwrap();
    assert_eq!(keyring.keys().await.next().unwrap(), TEST_KEYS[2]);

    let primary_key = keyring.primary_key().await;
    assert_eq!(primary_key.as_ref(), TEST_KEYS[2].as_ref());

    // Removing primary key should fail
    keyring.remove(&TEST_KEYS[2]).await.unwrap_err();

    // Removing non-primary key should succeed
    keyring.remove(&TEST_KEYS[1]).await.unwrap();
    assert_eq!(keyring.inner.read().await.keys.len() + 1, 1);
  }
}
