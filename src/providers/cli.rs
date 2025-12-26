use figment::{
    value::{Dict, Map, Value},
    Error, Profile, Provider,
};

pub struct CliProvider {
    overrides: Map<String, String>, // Store raw key-value pairs
}

impl CliProvider {
    /// Create provider from arguments iterator.
    /// Parses strings in "key=value" format.
    pub fn from_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut overrides = Map::new();

        for arg in args {
            let s = arg.as_ref();
            if let Some((key, value)) = s.split_once('=') {
                overrides.insert(key.to_string(), value.to_string());
            }
        }

        Self { overrides }
    }
}

impl Provider for CliProvider {
    fn metadata(&self) -> figment::Metadata {
        figment::Metadata::named("CLI Overrides")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        let mut data = Dict::new();

        for (key, value) in &self.overrides {
            let val = parse_value(value);
            insert_nested(&mut data, key, val);
        }

        let mut profiles = Map::new();
        profiles.insert(Profile::Default, data);
        Ok(profiles)
    }
}

fn parse_value(v: &str) -> Value {
    if let Ok(b) = v.parse::<bool>() {
        return Value::from(b);
    }
    if let Ok(i) = v.parse::<i64>() {
        return Value::from(i);
    }
    if let Ok(f) = v.parse::<f64>() {
        return Value::from(f);
    }
    Value::from(v)
}

fn insert_nested(map: &mut Dict, key: &str, value: Value) {
    if let Some((head, tail)) = key.split_once('.') {
        let entry = map
            .entry(head.to_string())
            .or_insert_with(|| Value::from(Dict::new()));

        // Handle Value::Dict(Tag, Dict)
        if let Value::Dict(_, ref mut inner_map) = entry {
            insert_nested(inner_map, tail, value);
        } else {
            // Conflict: overwrite with new dict
            let mut inner_map = Dict::new();
            insert_nested(&mut inner_map, tail, value);
            *entry = Value::from(inner_map);
        }
    } else {
        map.insert(key.to_string(), value);
    }
}
