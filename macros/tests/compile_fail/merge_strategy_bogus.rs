// MAC-09:非法 merge_strategy 值必须在宏展开期报错(而非静默忽略)。
use confers::Config;

#[derive(Debug, Config)]
struct Doc {
    #[config(merge_strategy = "bogus")]
    tags: Vec<String>,
}

fn main() {}
