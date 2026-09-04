use ahash::{AHashMap, AHashSet};

pub enum FlagKind {
    Bool,
    Value,
}

pub struct FlagSpec {
    pub names: &'static [&'static str],
    pub kind: FlagKind,
    pub description: Option<&'static str>,
}

impl FlagSpec {
    pub const fn flag(names: &'static [&'static str]) -> Self {
        Self {
            names,
            kind: FlagKind::Bool,
            description: None,
        }
    }

    pub const fn flag_value(names: &'static [&'static str]) -> Self {
        Self {
            names,
            kind: FlagKind::Value,
            description: None,
        }
    }

    pub const fn description(mut self, desc: &'static str) -> Self {
        self.description = Some(desc);
        self
    }

    pub fn get_description(&self) -> String {
        let names = self.names.join(", ");

        match self.description {
            Some(desc) => format!("[{names}]\n{desc}"),
            None => format!("[{names}]"),
        }
    }
}

pub struct Args<'a> {
    positional: Vec<String>,
    bools: AHashSet<&'static str>,
    values: AHashMap<&'static str, Option<String>>,
    flags: &'a [FlagSpec],
}

impl<'a> Args<'a> {
    pub fn parse(raw: &[String], specs: &'a [FlagSpec]) -> Self {
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

                            if value.is_empty() {
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
            flags: specs,
        }
    }

    pub fn get_flag_description(&self, name: &str) -> String {
        self.flags
            .iter()
            .find(|flag| flag.names.contains(&name))
            .map(FlagSpec::get_description)
            .expect("Unkown flag")
    }

    pub fn get_all_flags_description(&self) -> String {
        self.flags
            .iter()
            .map(FlagSpec::get_description)
            .collect::<Vec<_>>()
            .join("\n\n")
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
