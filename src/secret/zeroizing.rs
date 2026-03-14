use zeroize::Zeroizing;

pub type ZeroizingBytes = Zeroizing<Vec<u8>>;

pub fn zeroizing_bytes(bytes: Vec<u8>) -> ZeroizingBytes {
    Zeroizing::new(bytes)
}
