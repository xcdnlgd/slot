use std::fs;

use slot_store::{external_shaders, Platform};

fn touch(root: &std::path::Path, key: &str) {
    let path = root.join("Shaders").join(key);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

#[test]
fn a_platform_sees_common_and_its_own_folder_only() {
    let d = tempfile::tempdir().unwrap();
    touch(d.path(), "Common/common.glslp");
    touch(d.path(), "GBA/gba.glsl");
    touch(d.path(), "GB/gb.glsl");
    touch(d.path(), "GBC/gbc.glsl");

    let names = |p| {
        external_shaders(d.path(), p)
            .into_iter()
            .map(|s| s.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(names(Platform::Gba), ["common", "gba"]);
    assert_eq!(names(Platform::Gb), ["common", "gb"]);
    assert_eq!(names(Platform::Gbc), ["common", "gbc"]);
}

#[test]
fn a_platform_folder_wins_over_common_and_a_preset_wins_over_a_source() {
    let d = tempfile::tempdir().unwrap();
    touch(d.path(), "Common/same.glsl");
    touch(d.path(), "Common/preset.glsl");
    touch(d.path(), "Common/preset.glslp");
    touch(d.path(), "GBA/same.glslp");

    let found = external_shaders(d.path(), Platform::Gba);
    assert_eq!(found.len(), 2);
    let same = found.iter().find(|s| s.name == "same").unwrap();
    assert_eq!(same.key, "GBA/same.glslp");
    let preset = found.iter().find(|s| s.name == "preset").unwrap();
    assert_eq!(preset.key, "Common/preset.glslp");
}
