// rename_all 只接受 camelCase / snake_case / kebab-case,其余风格宏展开期报错。
use confers::Config;

#[derive(Debug, Config)]
#[config(rename_all = "PascalCase")]
struct Doc {
    host: String,
}

fn main() {}
