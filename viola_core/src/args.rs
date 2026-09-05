use ahash::{AHashMap, AHashSet};
use whatsapp_rust::{serde, serde_json};

pub enum FlagKind {
    Bool,
    Value,
    List,
    Json,
}

pub struct FlagSpec {
    pub names: &'static [&'static str],
    pub kind: FlagKind,
    pub description: Option<&'static str>,
}

impl FlagSpec {
    pub const fn bool(names: &'static [&'static str]) -> Self {
        Self {
            names,
            kind: FlagKind::Bool,
            description: None,
        }
    }

    pub const fn value(names: &'static [&'static str]) -> Self {
        Self {
            names,
            kind: FlagKind::Value,
            description: None,
        }
    }

    pub const fn list(names: &'static [&'static str]) -> Self {
        Self {
            names,
            kind: FlagKind::List,
            description: None,
        }
    }

    pub const fn json(names: &'static [&'static str]) -> Self {
        Self {
            names,
            kind: FlagKind::Json,
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
    lists: AHashMap<&'static str, Vec<String>>,
    flags: &'a [FlagSpec],
}

impl<'a> Args<'a> {
    pub fn parse(raw: &[String], specs: &'a [FlagSpec]) -> Self {
        let mut positional = Vec::new();
        let mut bools = AHashSet::new();
        let mut lists = AHashMap::new();
        let mut values = AHashMap::new();

        let mut i = 0;
        'tokens: while i < raw.len() {
            let tok = raw[i].as_str();

            let Some(spec) = specs.iter().find(|s| s.names.contains(&tok)) else {
                positional.push(raw[i].clone());
                i += 1;
                continue;
            };

            let canonical = spec.names[0];

            match spec.kind {
                FlagKind::Bool => {
                    bools.insert(canonical);
                    i += 1;
                }
                FlagKind::Value => {
                    let val = Self::collect_until_flag(raw, specs, &mut i);
                    values.insert(canonical, val);
                }
                FlagKind::List => {
                    i += 1;

                    let mut values = Vec::new();

                    while i < raw.len() {
                        let current = raw[i].as_str();

                        let is_flag = specs.iter().any(|s| s.names.contains(&current));

                        if is_flag {
                            break;
                        }

                        values.push(raw[i].clone());
                        i += i;
                    }

                    lists
                        .entry(canonical)
                        .or_insert_with(|| Vec::new())
                        .extend(values);
                }
                FlagKind::Json => {
                    let val = Self::collect_until_flag(raw, specs, &mut i);
                    values.insert(canonical, val);
                }
            }
            continue 'tokens;
        }

        Self {
            positional,
            bools,
            values,
            lists,
            flags: specs,
        }
    }

    fn collect_until_flag(raw: &[String], specs: &[FlagSpec], i: &mut usize) -> Option<String> {
        *i += 1;

        let start = *i;

        while *i < raw.len() {
            let current = raw[*i].as_str();

            let is_flag = specs.iter().any(|s| s.names.contains(&current));

            if is_flag {
                break;
            }

            *i += 1;
        }

        if start == *i {
            None
        } else {
            Some(raw[start..*i].join(" "))
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
        self.values.contains_key(canonical)
            || self.lists.contains_key(canonical)
            || self.bools.contains(canonical)
    }

    pub fn value(&self, canonical: &str) -> Option<&str> {
        self.values.get(canonical).and_then(|v| v.as_deref())
    }

    pub fn value_parsed<T: std::str::FromStr>(&self, canonical: &str) -> Option<T> {
        self.value(canonical).and_then(|v| v.parse().ok())
    }

    pub fn list(&self, canonical: &str) -> Option<&[String]> {
        self.lists.get(canonical).map(Vec::as_slice)
    }

    pub fn list_str(&self, canonical: &str) -> Option<Vec<&str>> {
        self.list(canonical)
            .map(|v| v.iter().map(String::as_str).collect())
    }

    pub fn json(&self, canonical: &str) -> Option<serde_json::Result<serde_json::Value>> {
        self.value(canonical).map(serde_json::from_str)
    }

    pub fn json_parsed<T>(&self, canonical: &str) -> Option<serde_json::Result<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.value(canonical).map(serde_json::from_str)
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
