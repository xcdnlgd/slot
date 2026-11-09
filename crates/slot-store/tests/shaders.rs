use std::fs;

use slot_store::{external_shaders, resolve_shader, seed_builtin_shaders, Platform};

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

#[test]
fn seeding_writes_the_builtins_and_never_overwrites_an_edit() {
    let d = tempfile::tempdir().unwrap();
    seed_builtin_shaders(d.path());
    for name in ["Grid", "LCD3x", "Dot", "Simpletex"] {
        assert!(
            d.path()
                .join(format!("Shaders/Common/{name}.glslp"))
                .is_file(),
            "{name} was not seeded"
        );
        assert!(
            d.path()
                .join(format!("Shaders/Common/shaders/{name}.glsl"))
                .is_file(),
            "{name} has no source"
        );
    }
    let names = external_shaders(d.path(), Platform::Gba)
        .into_iter()
        .map(|s| s.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["Dot", "Grid", "LCD3x", "Simpletex"]);

    std::fs::write(d.path().join("Shaders/Common/Grid.glslp"), "mine").unwrap();
    seed_builtin_shaders(d.path());
    assert_eq!(
        std::fs::read_to_string(d.path().join("Shaders/Common/Grid.glslp")).unwrap(),
        "mine"
    );
}

#[test]
fn resolving_a_stem_prefers_the_platform_folder_and_a_preset() {
    let d = tempfile::tempdir().unwrap();
    touch(d.path(), "Common/Grid.glsl");
    touch(d.path(), "GBA/Grid.glslp");
    let found = resolve_shader(d.path(), Platform::Gba, "Grid").unwrap();
    assert_eq!(found.key, "GBA/Grid.glslp");
    assert_eq!(found.name, "Grid");

    let common = resolve_shader(d.path(), Platform::Gb, "Grid").unwrap();
    assert_eq!(common.key, "Common/Grid.glsl");
}

#[test]
fn a_stem_that_names_a_path_does_not_resolve() {
    let d = tempfile::tempdir().unwrap();
    touch(d.path(), "Common/Grid.glsl");
    assert!(resolve_shader(d.path(), Platform::Gba, "../Grid").is_none());
    assert!(resolve_shader(d.path(), Platform::Gba, "Common/Grid").is_none());
}
