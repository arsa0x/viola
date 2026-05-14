use std::{collections::BTreeMap, fs, path::Path};
use walkdir::WalkDir;

#[derive(Default)]
struct ModuleTree {
    children: BTreeMap<String, ModuleTree>,
}

fn insert_path(root: &mut ModuleTree, parts: &[String]) {
    if parts.is_empty() {
        return;
    }
    let child = root.children.entry(parts[0].clone()).or_default();
    insert_path(child, &parts[1..]);
}

fn render_tree(tree: &ModuleTree, output: &mut String, depth: usize) {
    let indent = "   ".repeat(depth);
    for (name, child) in &tree.children {
        if child.children.is_empty() {
            output.push_str(&format!("{}pub mod {}; \n", indent, name));
        } else {
            output.push_str(&format!("{}pub mod {} {{\n", indent, name));
            render_tree(child, output, depth + 1);
            output.push_str(&format!("{}}}\n", indent,));
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=src/commands");
    let mut root = ModuleTree::default();
    for entry in WalkDir::new("src/commands")
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension() != Some("rs".as_ref()) {
            continue;
        }
        let relative = path.strip_prefix("src/commands").unwrap();
        let parts: Vec<String> = relative
            .iter()
            .map(|p| p.to_string_lossy().replace(".rs", ""))
            .collect();
        if parts.last() == Some(&"mod".to_string()) {
            continue;
        }
        insert_path(&mut root, &parts);
    }
    let mut output = String::new();
    output.push_str("// AUTO GENERATED\n\n");
    render_tree(&root, &mut output, 0);
    fs::write(Path::new("src/commands/mod.rs"), output).unwrap();
}
