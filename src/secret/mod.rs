#[cfg(feature = "encryption")]
pub mod bytes;
#[cfg(feature = "encryption")]
pub mod crypto;
#[cfg(feature = "encryption")]
pub mod key_provider;
#[cfg(feature = "encryption")]
pub mod key_registry;
#[cfg(feature = "encryption")]
pub mod providers;
#[cfg(feature = "encryption")]
pub mod string;

#[cfg(feature = "encryption")]
pub use bytes::SecretBytes;
#[cfg(feature = "encryption")]
pub use crypto::{derive_field_key, CryptoError, NONCE_SIZE, XChaCha20Crypto};
#[cfg(feature = "encryption")]
pub use key_provider::{EnvKeyProvider, EnvKeyProviderBuilder, SecretKeyProvider};
#[cfg(feature = "encryption")]
pub use key_registry::{KeyCachePolicy, KeyRegistry, KeyRegistryBuilder, KeyRotationConfig, KeyVersion};
#[cfg(feature = "encryption")]
pub use providers::{FileKeyProvider, FileKeyProviderBuilder};
#[cfg(all(feature = "encryption", feature = "remote"))]
pub use providers::{VaultKeyProvider, VaultKeyProviderBuilder};
#[cfg(feature = "encryption")]
pub use string::SecretString;
