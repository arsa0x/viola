use ahash::{AHashMap, AHashSet};

pub enum FlagKind {
    Bool,
    Value,
}

pub struct FlagSpec {
    pub names: &'static [&'static str],
    pub kind: FlagKind,
}

pub const fn flag(names: &'static [&'static str]) -> FlagSpec {
    FlagSpec {
        names,
        kind: FlagKind::Bool,
    }
}
pub const fn flag_value(names: &'static [&'static str]) -> FlagSpec {
    FlagSpec {
        names,
        kind: FlagKind::Value,
    }
}

pub struct Args {
    positional: Vec<String>,
    bools: AHashSet<&'static str>,
    values: AHashMap<&'static str, Option<String>>,
}

impl Args {
    pub fn parse(raw: &[String], specs: &[FlagSpec]) -> Self {
        let mut positional = Vec::new();
        let mut bools = AHashSet::new();
        let mut values = AHashMap::new();

        let mut i = 0;
        'tokens: while i < raw.len() {
            let tok = raw[i].as_str();

            for spec in specs {
                if spec.names.contains(&tok) {
                    let canonical = spec.names[0];
                    match spec.kind {
                        FlagKind::Bool => {
                            bools.insert(canonical);
                        }
                        FlagKind::Value => {
                            let mut value = Vec::new();
                            i += 1;

                            while i < raw.len() {
                                let current = raw[i].as_str();

                                let is_flag = specs.iter().any(|s| s.names.contains(&current));

                                if is_flag {
                                    break;
                                }

                                value.push(raw[i].clone());
                                i += 1;
                            }

                            if !value.is_empty() {
                                values.insert(canonical, None);
                            } else {
                                values.insert(canonical, Some(value.join(" ")));
                            }

                            continue 'tokens;
                        }
                    }
                    i += 1;
                    continue 'tokens;
                }
            }

            positional.push(raw[i].clone());
            i += 1;
        }

        Self {
            positional,
            bools,
            values,
        }
    }

    pub fn flag(&self, canonical: &str) -> bool {
        self.bools.contains(canonical)
    }

    pub fn has(&self, canonical: &str) -> bool {
        self.values.contains_key(canonical) || self.bools.contains(canonical)
    }

    pub fn value(&self, canonical: &str) -> Option<&str> {
        self.values.get(canonical).and_then(|v| v.as_deref())
    }

    pub fn value_parsed<T: std::str::FromStr>(&self, canonical: &str) -> Option<T> {
        self.value(canonical).and_then(|v| v.parse().ok())
    }

    pub fn positional(&self, idx: usize) -> Option<&str> {
        self.positional.get(idx).map(String::as_str)
    }

    pub fn rest(&self) -> String {
        self.positional.join(" ")
    }

    pub fn rest_from(&self, idx: usize) -> String {
        let start = idx.min(self.positional.len());
        self.positional[start..].join(" ")
    }
}
