#![allow(dead_code, unused_variables)]
use std::{
    io::{Read as _, Write as _},
    path::{NormalizeError, Path, PathBuf},
};

use code_modifier::{LanguageExt as _, langs::rust::Rust};
use mistral_nvim_derive::{Tool, ToolList};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{mistral::model::tools::*, notify::IntoNotification as _, nvim::model::SharedState};

fn get_path(file: &str) -> crate::Result<PathBuf> {
    let file_path = Path::new(&file);
    if !file_path.is_relative() {
        return Err("The file's path must be relative.".into_warn());
    }
    let current_path = Path::new(".").canonicalize()?;
    // let target = current_path.join(&file_path);
    let target = std::path::absolute(file_path)?
        .normalize_lexically()
        .map_err(|err: NormalizeError| "Can't normalize the path (lexically).".into_error())?;
    if !target.starts_with(current_path) {
        return Err("The file can't be outside the project directory.".into_warn());
    }
    Ok(target)
}

fn tree_git(target_dir: &str) -> crate::Result<String> {
    use gix::ThreadSafeRepository;
    let repo = ThreadSafeRepository::open(".").map_err(|e| e.to_string().into_warn())?;
    let index = repo
        .to_thread_local()
        .index()
        .map_err(|e| e.to_string().into_warn())?;
    // let target_dir = "src/mymodule/"; // Dossier cible

    // Collecter, filtrer et trier les chemins
    let paths: Vec<String> = index
        .entries()
        .into_iter()
        .map(|entry| entry.path(&index).to_string())
        .filter(|p| p.starts_with(target_dir)) // Retire le préfixe
        .map(|p| p.strip_prefix(&target_dir).unwrap().to_string()) // Retire le préfixe
        .collect();

    Ok(build_tree(target_dir, paths))
}

fn build_tree(target_dir: &str, mut paths: Vec<String>) -> String {
    paths.sort();
    use std::{collections::BTreeMap, path::Component};
    enum FileType<'c> {
        Dir(Node<'c>),
        File,
    }
    impl<'c> FileType<'c> {
        fn push_file(&mut self, components: impl Iterator<Item = Component<'c>>, file: Component<'c>) {
            match self {
                FileType::Dir(node) => node.push_file(components, file),
                FileType::File => (),
            }
        }
        fn tree(self, tree: &mut String, directory: &str, prefix: &str, is_last: bool) {
            let current_prefix = if is_last { "└── " } else { "├── " };
            let formatted_directory: std::borrow::Cow<_> = match &self {
                FileType::Dir(_) => format!("{directory}/").into(),
                _ => directory.into(),
            };
            let line = format!("{}{}{}\n", prefix, current_prefix, formatted_directory);
            tree.push_str(&line);
            match self {
                FileType::Dir(node) => {
                    let next_prefix = if is_last { "    " } else { "│   " };
                    let next_prefix = format!("{}{}", prefix, next_prefix);
                    node.tree(tree, directory, &next_prefix);
                }
                _ => (),
            }
        }
    }
    struct Node<'c> {
        nodes: BTreeMap<Component<'c>, FileType<'c>>,
    }
    impl<'c> Node<'c> {
        fn push_file(&mut self, mut components: impl Iterator<Item = Component<'c>>, file: Component<'c>) {
            if let Some(directory) = components.next() {
                use std::collections::btree_map::Entry;
                match self.nodes.entry(directory) {
                    Entry::Vacant(vacant_entry) => {
                        let mut created = Self {
                            nodes: Default::default(),
                        };
                        created.push_file(components, file);
                        vacant_entry.insert(FileType::Dir(created));
                    }
                    Entry::Occupied(mut node) => node.get_mut().push_file(components, file),
                }
            } else {
                self.nodes.insert(file, FileType::File);
            }
        }
        fn tree(self, tree: &mut String, directory: &str, prefix: &str) {
            let nb_nodes = self.nodes.len();
            for (i, (directory, node)) in self.nodes.into_iter().enumerate() {
                let Some(directory) = directory.as_os_str().to_str() else {
                    continue;
                };
                node.tree(tree, directory, &prefix, i == nb_nodes - 1);
            }
        }
    }
    let mut root = Node {
        nodes: Default::default(),
    };
    for path_str in paths.iter() {
        let path = Path::new(path_str);
        let mut components: Vec<_> = path.components().collect();
        let Some(file) = components.pop() else {
            continue;
        };
        root.push_file(components.into_iter(), file);
    }
    let mut tree = String::new();
    let target_dir = if target_dir.is_empty() { "." } else { target_dir };
    tree.push_str(target_dir);
    tree.push_str("\n");
    root.tree(&mut tree, target_dir, "");
    tree
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_refacto_get_file() -> crate::Result<()> {
        let path = get_path("..");
        assert!(path.is_err(), "{path:?}");
        let error = path.unwrap_err().to_string();
        assert!(
            error.ends_with("Warn : The file can't be outside the project directory."),
            "{error}"
        );
        let path = get_path("NON_EXISTING_FILE_...");
        assert!(path.is_ok());
        let path = path.unwrap();
        let path = path.to_string_lossy();
        assert!(path.ends_with("NON_EXISTING_FILE_..."), "{path:?}");
        Ok(())
    }
    #[test]
    fn test_build_tree() {
        let files = [
            "1", "a/b/c/2", "a/1", "a/b/c/1", "a/2", "2", "b/1", "b/2", "b/c/1", "b/c/2",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
        let tree = build_tree("src/", files);
        let expected = "src/
├── 1
├── 2
├── a/
│   ├── 1
│   ├── 2
│   └── b/
│       └── c/
│           ├── 1
│           └── 2
└── b/
    ├── 1
    ├── 2
    └── c/
        ├── 1
        └── 2
";
        crate::log_libuv!(Trace, "\nParsed:\n{tree}");
        crate::log_libuv!(Trace, "\nExpected:\n{expected}");
        // assert_eq!(tree, expected);
        for (line_modif, line_expected) in tree.split("\n").zip(expected.split("\n")) {
            assert_eq!(line_modif, line_expected, "\n\nDoes not match final.\n");
        }
    }
}

/// Permet de modifier de le code d'un fichier de manière granulaire.
/// Par exemple, si une fonction existe déjà elle sera remplacée si non, elle sera ajoutée.
/// Un formatteur puis un vérificateur seront lancés après ajout.
/// Les fichiers/dossiers qui n'existent pas seront créé.
#[derive(Serialize, Deserialize, Tool, JsonSchema)]
pub struct CodeModifier {
    /// Chemin relatif à la du racine projet.
    file: String,
    /// Code à ajouter.
    code: String,
}

impl Runnable for CodeModifier {
    type Ok = String;
    type Err = crate::notify::Notification;
    fn run(&mut self, state: SharedState, msg: crate::messages::RunToolMessage) -> Result<Self::Ok, Self::Err> {
        let target = get_path(&self.file)?;
        if target.exists() {
            let mut file = std::fs::File::options()
                .read(true)
                .write(true)
                .truncate(true)
                .open(&target)?;
            let mut code = Vec::new();
            let code_injection = self.code.as_bytes();
            file.read_to_end(&mut code)?;
            let parser = match target.extension().map(|os_str| os_str.to_str()) {
                Some(Some("rs")) => Rust::new_parser(code.as_slice()),
                _ => {
                    file.write_all(code_injection)?;
                    return Ok(
                        "File fully replaced : file's extension not supported for granular updates.".to_string(),
                    );
                }
            };
            let Some(mut parser) = parser else {
                // If failed write back previous content.
                file.write_all(code.as_slice())?;
                return Err("Can't parser code with tree-sitter.".into_error());
            };
            let Some(modifications) = parser.inject(code_injection) else {
                // If failed write back previous content.
                file.write_all(code.as_slice())?;
                return Err("Can't parser code with tree-sitter.".into_error());
            };
            modifications.apply_injections(&mut code, code_injection);
            file.write_all(code.as_slice())?;
            Ok("File updated.".to_string())
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::File::options()
                .write(true)
                .create_new(true)
                .open(target)?;
            file.write_all(self.code.as_bytes())?;
            Ok("File created.".to_string())
        }
    }
}

/// Permet de récupérer le contenu des fichiers d'un projet.
#[derive(Serialize, Deserialize, Tool, JsonSchema, PartialEq, Eq, Debug)]
pub struct CodeRetriever {
    /// Chemin relatif à la du racine projet.
    pub file: String,
}

impl Runnable for CodeRetriever {
    type Ok = String;
    type Err = crate::notify::Notification;
    fn run(&mut self, state: SharedState, msg: crate::messages::RunToolMessage) -> Result<Self::Ok, Self::Err> {
        let target = get_path(&self.file)?;
        if target.exists() {
            let mut file = std::fs::File::options().read(true).open(&target)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            Ok(content)
        } else {
            Err("File does not exist.".into_warn())
        }
    }
}

/// Récupère le arbre des fichiers d'un dossier.
/// Équivalent à la commande bash `tree`.
#[derive(Serialize, Deserialize, Tool, JsonSchema)]
pub struct CodeTree {
    /// Chemin relatif à la du racine projet. Permettant de cibler un dossier précis.
    directory: Option<String>,
}

impl Runnable for CodeTree {
    type Ok = String;
    type Err = crate::notify::Notification;
    fn run(&mut self, state: SharedState, msg: crate::messages::RunToolMessage) -> Result<Self::Ok, Self::Err> {
        let target = if let Some(dir) = self.directory.as_ref() {
            get_path(&dir)?;
            dir.as_str()
        } else {
            ""
        };
        tree_git(target)
    }
}

#[derive(ToolList)]
pub struct CodeRefactorisation(CodeModifier, CodeRetriever, CodeTree);
