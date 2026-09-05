// MAC-17:env_prefix 含非法字符必须在宏展开期报错。
use confers::Config;

#[derive(Debug, Config)]
#[config(env_prefix = "bad-prefix")]
struct Doc {
    host: String,
}

fn main() {}
