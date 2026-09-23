use confers::Config;

#[derive(Config)]
struct AesGcmProbe {
    #[config(encrypt = "aes256-gcm")]
    pub api_key: String,
}

fn main() {}
