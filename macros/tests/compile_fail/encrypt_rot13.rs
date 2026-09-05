// ENC-23:非法加密算法名必须在宏展开期报错并列出支持的算法。
use confers::Config;

#[derive(Debug, Config)]
struct Doc {
    #[config(encrypt = "rot13")]
    secret: String,
}

fn main() {}
