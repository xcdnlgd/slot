use std::path::Path;

use slot_gfx::load_shader;

#[test]
fn every_builtin_preset_loads_its_single_pass() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../card/Shaders/Common");
    for name in ["Grid", "LCD3x", "Dot", "Simpletex"] {
        let path = dir.join(format!("{name}.glslp"));
        let shader = load_shader(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            shader.text.contains("#if defined(VERTEX)"),
            "{name} has no vertex stage"
        );
        assert!(
            shader.text.contains("#elif defined(FRAGMENT)"),
            "{name} has no fragment stage"
        );
        assert!(!shader.linear, "{name} must sample nearest");
    }
}
