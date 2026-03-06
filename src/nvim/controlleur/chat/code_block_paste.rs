/// Cherche les block et écrit lure contenu vers le fichier ciblé, en demandant la validation via
/// getchar.
use std::{
    fs::OpenOptions,
    io::{self, Write as _},
};

use nvim_oxi::api::{self, types::CommandArgs};

use crate::nvim::model::{BufferData, Selection};

pub(super) fn code_block_paste(args: CommandArgs) -> crate::Result<()> {
    let (data, _buffer) = BufferData::from_current_buffer()?;
    let selection = Selection::from_command_args(&args);
    let visual = crate::messages::Visual { data, selection };
    let content = visual.get_selected_content().0;

    if content.is_empty() {
        return Ok(());
    }
    parse_and_write(content)?;
    Ok(())
}

fn parse_and_write(content: impl ToString) -> crate::Result<()> {
    let content = content.to_string();
    let mut buf = Vec::new();
    let mut in_block = false;
    let mut _block = None;
    let mut path = None;
    let mut all_files_blicks = Vec::default();
    for line in content.lines() {
        if line.starts_with("```") {
            if !in_block {
                in_block = true;
                let name = &line[3..];
                if !name.is_empty() {
                    _block = Some(name);
                }
            } else {
                in_block = false;
                if let Some(path) = path.take() {
                    all_files_blicks.push((path, buf.drain(..).collect::<Vec<_>>()));
                }
            }
        } else if in_block {
            buf.extend((line.to_string() + "\n").as_bytes())
        } else {
            let mut it_chars = line.chars();
            while let Some(c) = it_chars.next() {
                if c == '`' {
                    break;
                }
            }
            let mut path_str = String::new();
            while let Some(c) = it_chars.next() {
                if c == '`' {
                    break;
                } else {
                    path_str.push(c);
                }
            }
            if path_str != "" {
                path = Some(path_str);
            }
        }
    }
    crate::notify::info("\nList of changmeents :");
    let mut dirs_missing = Vec::new();
    let current_dir = std::fs::canonicalize(std::path::Path::new("."))?;
    for (path, _buf) in &all_files_blicks {
        let path = current_dir.join(path);
        crate::notify::info(format!("    - {}", path.to_string_lossy()));
        let Some(dir) = path.parent() else {
            continue;
        };
        if !dir.exists() {
            dirs_missing.push(dir.to_path_buf());
        }
    }
    if !dirs_missing.is_empty() {
        crate::notify::info("\nList of dirs that will be creaed :");
        for dir in &dirs_missing {
            crate::notify::info(format!("    - {}", dir.to_string_lossy()));
        }
    }
    let prepend_question = format!("\nShould we write these files ?");
    let prompt = get_prompt(prepend_question)?;
    for (path, mut buf) in all_files_blicks {
        if write_file(prompt, dirs_missing.drain(..), (path, &mut buf))?.is_none() {
            break;
        };
    }
    Ok(())
}

fn get_prompt(prepend_question: String) -> crate::Result<char> {
    let question = format!("{prepend_question}\n [Aa]ppend, [Tt]runcate, [Nn]o ; Majuscule to use the same until end");
    crate::notify::info(question);
    let prompt: nvim_oxi::String = api::eval("nr2char(getchar())")?;
    Ok(prompt.to_string().chars().next().unwrap_or('n'))
}

fn write_file(
    prompt: char,
    dirs_missing: impl Iterator<Item = std::path::PathBuf>,
    (path, buf): (String, &mut Vec<u8>),
) -> io::Result<Option<char>> {
    let mut truncate = false;
    let mut append = false;
    match prompt {
        'A' | 'a' => append = true,
        'T' | 't' => truncate = true,
        'N' | 'n' => (),
        _ => {
            crate::notify::info("Not valide repsonse, so skip the file.");
            return Ok(None);
        }
    }
    let keep_answer = if prompt.is_ascii_uppercase() {
        Some(prompt)
    } else {
        None
    };
    if !append && !truncate {
        return Ok(keep_answer);
    }
    for dir in dirs_missing {
        std::fs::create_dir_all(dir)?;
    }
    let file_content = buf.drain(..);
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(truncate)
        .append(append)
        .create(true)
        .open(path)?;
    file.write_all(file_content.as_slice())?;
    Ok(keep_answer)
}
