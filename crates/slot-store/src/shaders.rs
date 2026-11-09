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
        here.sort_by_key(|s| s.name.to_ascii_lowercase());
        for shader in here {
            if let Some(at) = found
                .iter()
                .position(|s| s.name.eq_ignore_ascii_case(&shader.name))
            {
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
    found.sort_by_key(|s| s.name.to_ascii_lowercase());
    found
}

/// Find the file a stored shader stem names. The platform folder wins over
/// Common, and a preset wins over a bare source.
pub fn resolve_shader(root: &Path, platform: Platform, name: &str) -> Option<ExternalShader> {
    if !valid_shader_stem(name) {
        return None;
    }
    let platform = platform.hardware();
    for folder in [platform.dir_name(), "Common"] {
        for ext in ["glslp", "glsl"] {
            let file = format!("{name}.{ext}");
            let path = root.join("Shaders").join(folder).join(&file);
            if path.is_file() {
                return Some(ExternalShader {
                    name: name.to_string(),
                    key: format!("{folder}/{file}"),
                    path,
                });
            }
        }
    }
    None
}

/// A stored shader key is a file stem, not a path. Keep it from escaping the
/// shader folders when it comes back from a config file.
pub fn valid_shader_stem(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\'])
        && !name.eq_ignore_ascii_case("off")
        && !name.eq_ignore_ascii_case("external")
}

const BUILTIN_SHADERS: [(&str, &str); 8] = [
    (
        "Grid.glslp",
        include_str!("../../../card/Shaders/Common/Grid.glslp"),
    ),
    (
        "shaders/Grid.glsl",
        include_str!("../../../card/Shaders/Common/shaders/Grid.glsl"),
    ),
    (
        "LCD3x.glslp",
        include_str!("../../../card/Shaders/Common/LCD3x.glslp"),
    ),
    (
        "shaders/LCD3x.glsl",
        include_str!("../../../card/Shaders/Common/shaders/LCD3x.glsl"),
    ),
    (
        "Dot.glslp",
        include_str!("../../../card/Shaders/Common/Dot.glslp"),
    ),
    (
        "shaders/Dot.glsl",
        include_str!("../../../card/Shaders/Common/shaders/Dot.glsl"),
    ),
    (
        "Simpletex.glslp",
        include_str!("../../../card/Shaders/Common/Simpletex.glslp"),
    ),
    (
        "shaders/Simpletex.glsl",
        include_str!("../../../card/Shaders/Common/shaders/Simpletex.glsl"),
    ),
];

/// Older cards shipped without the built-in shader files. Write the missing
/// ones back on boot, but never overwrite a file the user has edited.
pub fn seed_builtin_shaders(root: &Path) {
    let common = root.join("Shaders/Common");
    let _ = std::fs::create_dir_all(common.join("shaders"));
    for (name, text) in BUILTIN_SHADERS {
        let path = common.join(name);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = crate::atomic_write(&path, text.as_bytes());
    }
}
