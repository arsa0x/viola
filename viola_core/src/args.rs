use std::fmt;

use ahash::{AHashMap, AHashSet};
use serde;
use whatsapp_rust::serde_json;

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
            Some(desc) => format!("```[{names}]```\n> {desc}"),
            None => format!("```[{names}]```"),
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
                        i += 1;
                    }

                    lists.entry(canonical).or_insert_with(|| Vec::new()).extend(
                        values
                            .iter()
                            .flat_map(|v| {
                                v.split(",")
                                    .map(str::trim)
                                    .filter(|val| !val.is_empty())
                                    .map(ToOwned::to_owned)
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                FlagKind::Json => {
                    let val = Self::collect_json(raw, specs, &mut i);
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

    fn canonical_name(&self, name: &str) -> Option<&'static str> {
        self.flags
            .iter()
            .find(|f| f.names.contains(&name))
            .map(|f| f.names[0])
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
            .expect("Unknown flag")
    }

    pub fn get_all_flags_description(&self) -> String {
        self.flags
            .iter()
            .map(FlagSpec::get_description)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn flag(&self, canonical: &str) -> bool {
        self.canonical_name(canonical)
            .map(|c| self.bools.contains(c))
            .unwrap_or(false)
    }

    pub fn has(&self, canonical: &str) -> bool {
        let Some(c) = self.canonical_name(canonical) else {
            return false;
        };

        self.values.contains_key(c) || self.lists.contains_key(c) || self.bools.contains(c)
    }

    pub fn value(&self, canonical: &str) -> Option<&str> {
        let c = self.canonical_name(canonical)?;

        self.values.get(c).and_then(|v| v.as_deref())
    }

    pub fn value_parsed<T: std::str::FromStr>(&self, canonical: &str) -> Option<T> {
        self.value(canonical).and_then(|v| v.parse().ok())
    }

    pub fn list(&self, canonical: &str) -> Option<&[String]> {
        let c = self.canonical_name(canonical)?;

        self.lists.get(c).map(Vec::as_slice)
    }

    pub fn list_str(&self, canonical: &str) -> Option<Vec<&str>> {
        self.list(canonical)
            .map(|v| v.iter().map(String::as_str).collect())
    }

    pub fn list_non_empty(&self, canonical: &str) -> Option<Vec<&str>> {
        self.list_str(canonical).filter(|v| !v.is_empty())
    }

    fn collect_json(raw: &[String], specs: &[FlagSpec], i: &mut usize) -> Option<String> {
        *i += 1;

        let start = *i;
        let mut end = start;

        while end < raw.len() {
            let current = raw[end].as_str();

            if end > start && specs.iter().any(|s| s.names.contains(&current)) {
                break;
            }

            end += 1;

            let candidate = raw[start..end].join(" ");

            if serde_json::from_str::<serde_json::Value>(&candidate).is_ok() {
                *i = end;
                return Some(candidate);
            }
        }

        if start == end {
            *i = end;
            None
        } else {
            *i = end;
            Some(raw[start..end].join(" "))
        }
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

impl fmt::Debug for Args<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Args")
            .field("positional", &self.positional)
            .field("bools", &self.bools)
            .field("values", &self.values)
            .field("lists", &self.lists)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specs() -> Vec<FlagSpec> {
        vec![
            FlagSpec::bool(&["-v", "--verbose"]),
            FlagSpec::value(&["-n", "--name"]),
            FlagSpec::list(&["-t", "--tag"]),
            FlagSpec::json(&["-c", "--config"]),
        ]
    }

    fn args(input: &[&str]) -> Args<'static> {
        let specs = Box::leak(Box::new(specs()));

        let raw = input
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();

        Args::parse(&raw, specs)
    }

    #[test]
    fn parses_bool_flag() {
        let args = args(&["--verbose"]);

        assert!(args.flag("--verbose"));
        assert!(args.has("--verbose"));
    }

    #[test]
    fn parses_bool_flag_using_alias() {
        let args = args(&["-v"]);

        assert!(args.flag("-v"));
        assert!(args.flag("--verbose"));
        assert!(args.has("--verbose"));
    }

    #[test]
    fn bool_flag_defaults_to_false() {
        let args = args(&[]);

        assert!(!args.flag("--verbose"));
        assert!(!args.has("--verbose"));
    }

    #[test]
    fn parses_single_value() {
        let args = args(&["--name", "arsa", "--verbose"]);

        assert_eq!(args.value("--name"), Some("arsa"));
        assert!(args.flag("--verbose"));
    }

    #[test]
    fn parses_value_until_next_flag() {
        let args = args(&["--name", "hello", "world", "--verbose"]);

        assert_eq!(args.value("--name"), Some("hello world"));
        assert!(args.flag("--verbose"));
    }

    #[test]
    fn value_without_content_returns_none() {
        let args = args(&["--name", "--verbose"]);

        assert!(args.has("--name"));
        assert_eq!(args.value("--name"), None);
    }

    #[test]
    fn parses_value_as_type() {
        let specs = Box::leak(Box::new(vec![FlagSpec::value(&["--port"])]));

        let raw = vec!["--port".to_string(), "8080".to_string()];

        let args = Args::parse(&raw, specs);

        let port: Option<u16> = args.value_parsed("--port");

        assert_eq!(port, Some(8080));
    }

    #[test]
    fn invalid_parsed_value_returns_none() {
        let specs = Box::leak(Box::new(vec![FlagSpec::value(&["--port"])]));

        let raw = vec!["--port".to_string(), "invalid".to_string()];

        let args = Args::parse(&raw, specs);

        let port: Option<u16> = args.value_parsed("--port");

        assert_eq!(port, None);
    }

    #[test]
    fn parses_list() {
        let args = args(&["--tag", "rust", "whatsapp", "bot", "--verbose"]);

        assert_eq!(
            args.list_str("--tag"),
            Some(vec!["rust", "whatsapp", "bot",])
        );

        assert!(args.flag("--verbose"));
    }

    #[test]
    fn parses_comma_separated_list() {
        let args = args(&["--tag", "rust,whatsapp,bot"]);

        assert_eq!(
            args.list_str("--tag"),
            Some(vec!["rust", "whatsapp", "bot",])
        );
    }

    #[test]
    fn parses_mixed_list_and_comma_separated_values() {
        let args = args(&["--tag", "rust, whatsapp", "bot,", "framework"]);

        assert_eq!(
            args.list_str("--tag"),
            Some(vec!["rust", "whatsapp", "bot", "framework",])
        );
    }

    #[test]
    fn trims_list_values() {
        let args = args(&["--tag", " rust ", " whatsapp ", "bot"]);

        assert_eq!(
            args.list_str("--tag"),
            Some(vec!["rust", "whatsapp", "bot",])
        );
    }

    #[test]
    fn ignores_empty_list_values() {
        let args = args(&["--tag", ",", ",rust,,", "whatsapp,,"]);

        assert_eq!(args.list_str("--tag"), Some(vec!["rust", "whatsapp",]));
    }

    #[test]
    fn repeated_list_flags_are_appended() {
        let args = args(&["--tag", "rust", "--tag", "whatsapp", "--tag", "bot"]);

        assert_eq!(
            args.list_str("--tag"),
            Some(vec!["rust", "whatsapp", "bot",])
        );
    }

    #[test]
    fn empty_list_flag_creates_empty_list() {
        let args = args(&["--tag", "--verbose"]);

        assert!(args.has("--tag"));
        assert_eq!(args.list_str("--tag"), Some(Vec::<&str>::new()));
    }

    #[test]
    fn parses_json_object() {
        let args = args(&["--config", r#"{"debug":true,"port":8080}"#]);

        let json = args
            .json("--config")
            .expect("JSON flag should exist")
            .expect("JSON should be valid");

        assert_eq!(json["debug"], true);
        assert_eq!(json["port"], 8080);
    }

    #[test]
    fn parses_json_array() {
        let args = args(&["--config", r#"[1,2,3]"#]);

        let json = args
            .json("--config")
            .expect("JSON flag should exist")
            .expect("JSON should be valid");

        assert_eq!(json.as_array().unwrap().len(), 3);
    }

    #[test]
    fn parses_multitoken_json() {
        let args = args(&[
            "--config",
            r#"{"name":"hello"#,
            r#"world","debug":true}"#,
            "--verbose",
        ]);

        let json = args
            .json("--config")
            .expect("JSON flag should exist")
            .expect("JSON should be valid");

        assert_eq!(json["name"], "hello world");
        assert_eq!(json["debug"], true);

        assert!(args.flag("--verbose"));
    }

    #[test]
    fn invalid_json_returns_error() {
        let args = args(&["--config", r#"{"debug":true"#]);

        let result = args.json("--config").expect("JSON flag should exist");

        assert!(result.is_err());
    }

    #[test]
    fn parses_json_into_struct() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Config {
            debug: bool,
            port: u16,
        }

        let args = args(&["--config", r#"{"debug":true,"port":8080}"#]);

        let config = args
            .json_parsed::<Config>("--config")
            .expect("JSON flag should exist")
            .expect("JSON should be valid");

        assert_eq!(
            config,
            Config {
                debug: true,
                port: 8080,
            }
        );
    }

    #[test]
    fn invalid_typed_json_returns_error() {
        #[derive(Debug, serde::Deserialize)]
        #[allow(unused)]
        struct Config {
            port: u16,
        }

        let args = args(&["--config", r#"{"port":"invalid"}"#]);

        let result = args
            .json_parsed::<Config>("--config")
            .expect("JSON flag should exist");

        assert!(result.is_err());
    }

    #[test]
    fn positional_arguments_are_preserved() {
        let args = args(&["hello", "world", "--verbose", "viola"]);

        assert_eq!(args.positional(0), Some("hello"));
        assert_eq!(args.positional(1), Some("world"));
        assert_eq!(args.positional(2), Some("viola"));
    }

    #[test]
    fn rest_returns_all_positional_arguments() {
        let args = args(&["hello", "world", "--verbose", "viola"]);

        assert_eq!(args.rest(), "hello world viola");
    }

    #[test]
    fn rest_from_returns_positional_arguments_after_index() {
        let args = args(&["hello", "world", "viola"]);

        assert_eq!(args.rest_from(0), "hello world viola");
        assert_eq!(args.rest_from(1), "world viola");
        assert_eq!(args.rest_from(2), "viola");
        assert_eq!(args.rest_from(99), "");
    }

    #[test]
    fn unknown_token_becomes_positional() {
        let args = args(&["--unknown", "value"]);

        assert_eq!(args.positional(0), Some("--unknown"));
        assert_eq!(args.positional(1), Some("value"));
    }

    #[test]
    fn mixed_flags_are_parsed_correctly() {
        let args = args(&[
            "--verbose",
            "--name",
            "arsa",
            "--tag",
            "rust,whatsapp",
            "bot",
            "--config",
            r#"{"debug":true,"port":8080}"#,
            "hello",
            "world",
        ]);

        assert!(args.flag("--verbose"));

        assert_eq!(args.value("--name"), Some("arsa"));

        assert_eq!(
            args.list_str("--tag"),
            Some(vec!["rust", "whatsapp", "bot",])
        );

        let json = args
            .json("--config")
            .expect("JSON flag should exist")
            .expect("JSON should be valid");

        assert_eq!(json["debug"], true);
        assert_eq!(json["port"], 8080);

        assert_eq!(args.rest(), "hello world");
    }

    #[test]
    fn flag_aliases_share_canonical_storage() {
        let args = args(&["-v", "-t", "rust"]);

        assert!(args.flag("--verbose"));
        assert!(args.has("--verbose"));

        assert!(args.has("--tag"));
        assert_eq!(args.list_str("--tag"), Some(vec!["rust"]));
    }

    #[test]
    fn descriptions_are_generated() {
        let flag = FlagSpec::bool(&["-v", "--verbose"]).description("Enable verbose mode");

        assert_eq!(
            flag.get_description(),
            "[-v, --verbose]\nEnable verbose mode"
        );
    }

    #[test]
    fn descriptions_without_text_are_generated() {
        let flag = FlagSpec::bool(&["-v", "--verbose"]);

        assert_eq!(flag.get_description(), "[-v, --verbose]");
    }
}
