use std::path::PathBuf;

use super::core::SearchMatch;

// --- Flat Row Enum for show_rows Virtual Rendering ---
pub enum FlatRow<'a> {
    Directory {
        node: &'a SearchTree,
        indent: usize,
    },
    File {
        node: &'a SearchTree,
        indent: usize,
    },
    Match {
        search_match: &'a SearchMatch,
        indent: usize,
    },
}

// --- Hierarchical Tree View Structure ---
#[derive(Clone)]
pub struct SearchTree {
    pub name: String,
    pub path: PathBuf,
    pub is_file: bool,
    pub children: std::collections::BTreeMap<String, Self>,
    pub matches: Vec<SearchMatch>,
    pub is_expanded: bool,
}

impl Default for SearchTree {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::new(),
            is_file: false,
            children: std::collections::BTreeMap::new(),
            matches: Vec::new(),
            is_expanded: true,
        }
    }
}

impl SearchTree {
    pub fn insert(&mut self, full_match: SearchMatch, relative_components: &[String]) {
        if relative_components.is_empty() {
            self.is_file = true;
            self.matches.push(full_match);
            return;
        }

        let name = relative_components[0].clone();
        let child_path = self.path.join(&name);

        let child = self.children.entry(name.clone()).or_insert_with(|| Self {
            name,
            path: child_path,
            is_file: false,
            children: std::collections::BTreeMap::new(),
            matches: Vec::new(),
            is_expanded: true,
        });

        child.insert(full_match, &relative_components[1..]);
    }

    pub fn flatten_impl<'a>(&'a self, indent: usize, out: &mut Vec<FlatRow<'a>>) {
        if self.name.is_empty() {
            // Root node: just recurse into children
            for child in self.children.values() {
                child.flatten_impl(indent, out);
            }
            return;
        }

        if self.is_file {
            out.push(FlatRow::File { node: self, indent });
            if self.is_expanded {
                for m in &self.matches {
                    out.push(FlatRow::Match {
                        search_match: m,
                        indent: indent + 1,
                    });
                }
            }
        } else {
            out.push(FlatRow::Directory { node: self, indent });
            if self.is_expanded {
                for child in self.children.values() {
                    child.flatten_impl(indent + 1, out);
                }
            }
        }
    }

    pub fn toggle_expanded(&mut self, target_path: &std::path::Path) {
        if self.path == target_path {
            self.is_expanded = !self.is_expanded;
            return;
        }
        for child in self.children.values_mut() {
            if target_path.starts_with(&child.path) {
                child.toggle_expanded(target_path);
                return;
            }
        }
    }
}
