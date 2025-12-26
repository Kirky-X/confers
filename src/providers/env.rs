// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use figment::providers::Env;
use figment::Figment;

pub struct EnvProvider {
    prefix: String,
}

impl EnvProvider {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    pub fn load(&self) -> Figment {
        Figment::from(Env::prefixed(&self.prefix).split("__"))
    }
}
