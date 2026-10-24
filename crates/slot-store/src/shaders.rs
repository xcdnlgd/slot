use std::path::{Path, PathBuf};

use crate::Platform;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalShader {
    pub name: String,
    pub key: String,
    pub path: PathBuf,
}

/// The shaders a platform may pick: Common, then its own folder. A platform
/// shader replaces a Common shader with the same stem, and a preset replaces a
/// bare source with the same stem.
pub fn external_shaders(root: &Path, platform: Platform) -> Vec<ExternalShader> {
    let shaders = root.join("Shaders");
    let mut found: Vec<ExternalShader> = Vec::new();
    for folder in ["Common", platform.dir_name()] {
        let Ok(entries) = std::fs::read_dir(shaders.join(folder)) else {
            continue;
        };
        let mut here: Vec<ExternalShader> = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_file() {
                    return None;
                }
                let ext = path.extension()?.to_str()?;
                let ext = ext.to_ascii_lowercase();
                if ext != "glsl" && ext != "glslp" {
                    return None;
                }
                let name = path.file_stem()?.to_str()?.to_string();
                if name.starts_with('.') {
                    return None;
                }
                let key = format!("{folder}/{}", path.file_name()?.to_str()?);
                Some(ExternalShader {
                    name,
                    path: shaders.join(&key),
                    key,
                })
            })
            .collect();
        here.sort_by(|a, b| a.name.cmp(&b.name));
        for shader in here {
            if let Some(at) = found.iter().position(|s| s.name == shader.name) {
                let old_ext = found[at].path.extension().and_then(|e| e.to_str());
                let new_ext = shader.path.extension().and_then(|e| e.to_str());
                let replaces = folder == platform.dir_name()
                    || (new_ext == Some("glslp") && old_ext == Some("glsl"));
                if replaces {
                    found[at] = shader;
                }
            } else {
                found.push(shader);
            }
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}
