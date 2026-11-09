mod common;

use common::{tmp_root_with_carts, tmp_root_with_gb_carts, tmp_root_with_gbc_carts, StubSnapshot};
use slot::app::{App, Phase};
use slot_gfx::WHOLE_TEXTURE;
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, BezelPick, Platform, Scale, SlotState};
use slot_ui::{BezelArt, QuickRow, QuickValue};
use tempfile::TempDir;

fn root_for(platform: Platform) -> TempDir {
    match platform {
        Platform::Gba => tmp_root_with_carts(&["Game", "Zzz"]),
        Platform::Gb => tmp_root_with_gb_carts(&["Game", "Zzz"]),
        Platform::Gbc => tmp_root_with_gbc_carts(&["Game", "Zzz"]),
        Platform::Gbav => {
            let d = tmp_root_with_carts(&["Zzz"]);
            let dir = d.path().join("Games").join(Platform::Gbav.dir_name());
            std::fs::create_dir_all(&dir).unwrap();
            let mut rom = vec![0u8; 0x100];
            rom[0xa0..0xa4].copy_from_slice(b"GAME");
            std::fs::write(dir.join("Game.gba"), rom).unwrap();
            d
        }
    }
}

fn playing(platform: Platform, panel: (u32, u32), state: SlotState) -> (TempDir, App) {
    let d = root_for(platform);
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Game".into()),
            cart_platform: Some(platform),
            clock_set: true,
            ..state
        },
    )
    .unwrap();
    let mut a = App::boot(d.path());
    a.set_panel(panel);
    a.set_snapshot(StubSnapshot::boxed());
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "not playing: {:?}",
        a.phase()
    );
    assert_eq!(a.playing_platform(), platform);
    (d, a)
}

#[test]
fn defaults_fit_each_platform_the_way_it_should() {
    for panel in [(720, 480), (640, 480)] {
        for platform in [Platform::Gba, Platform::Gb, Platform::Gbc] {
            let (_d, a) = playing(platform, panel, SlotState::default());
            let size = a.game_size();
            let offset = a.game_offset();
            let want = match (platform, panel) {
                (Platform::Gba, (720, 480)) => ((720, 480), (0, 0)),
                (Platform::Gba, _) => ((640, 427), (0, 26)),
                (_, (720, 480)) => ((480, 432), (120, 24)),
                _ => ((480, 432), (80, 24)),
            };
            assert_eq!((size.w, size.h), want.0, "{platform:?} {panel:?}");
            assert_eq!(offset, want.1, "{platform:?} {panel:?}");
            assert_eq!(a.source_rect(), WHOLE_TEXTURE, "{platform:?} {panel:?}");
        }
    }
}

#[test]
fn gb_integer_on_4_by_3_uses_the_whole_panel_height() {
    let (_d, a) = playing(
        Platform::Gb,
        (640, 480),
        SlotState {
            bezel_gb: BezelPick::Ags102,
            scale_gb: Scale::Integer,
            ..SlotState::default()
        },
    );
    assert_eq!((a.game_size().w, a.game_size().h), (480, 432));
    assert_eq!(a.game_offset(), (80, 24));
    assert_eq!(a.source_rect(), WHOLE_TEXTURE);
}

#[test]
fn gba_video_follows_the_gba_scale() {
    let (_d, a) = playing(
        Platform::Gbav,
        (640, 480),
        SlotState {
            scale_gba: Scale::FullScreen,
            ..SlotState::default()
        },
    );
    assert_eq!((a.game_size().w, a.game_size().h), (640, 480));
    assert_eq!(a.game_offset(), (0, 0));
    let (_d, a) = playing(Platform::Gbav, (640, 480), SlotState::default());
    assert_eq!((a.game_size().w, a.game_size().h), (640, 427));
    assert_eq!(a.game_offset(), (0, 26));
}

#[test]
fn in_game_rows_follow_the_playing_platform() {
    let (_d, a) = playing(Platform::Gba, (640, 480), SlotState::default());
    let hidden = a.in_game_hidden();
    assert!(!hidden.contains(&QuickRow::Scale));
    assert_eq!(a.quick_value(QuickRow::Scale), Some(QuickValue::Fit));
    let (_d, a) = playing(Platform::Gb, (720, 480), SlotState::default());
    let hidden = a.in_game_hidden();
    assert!(!hidden.contains(&QuickRow::Scale));
}

#[test]
fn the_shoulders_cycle_scale_only_on_game_boy() {
    let (d, mut a) = playing(Platform::Gbc, (640, 480), SlotState::default());
    assert_eq!(a.resolved_scale(Platform::Gbc), Scale::Integer);
    a.apply(Action::GbaDown(Btn::R1));
    a.apply(Action::GbaUp(Btn::R1));
    assert_eq!(read_slot_state(d.path()).scale_gbc, Scale::Fit);
    a.apply(Action::GbaDown(Btn::R1));
    a.apply(Action::GbaUp(Btn::R1));
    assert_eq!(read_slot_state(d.path()).scale_gbc, Scale::FullScreen);
    a.apply(Action::GbaDown(Btn::L1));
    a.apply(Action::GbaUp(Btn::L1));
    assert_eq!(read_slot_state(d.path()).scale_gbc, Scale::Fit);

    let (d, mut a) = playing(Platform::Gba, (640, 480), SlotState::default());
    let before = read_slot_state(d.path()).scale_gba;
    a.apply(Action::GbaDown(Btn::R1));
    a.apply(Action::GbaUp(Btn::R1));
    assert_eq!(read_slot_state(d.path()).scale_gba, before);
}

#[test]
fn bezel_art_follows_the_layout() {
    for (panel, scale, want) in [
        ((720, 480), Scale::Fit, Some(BezelArt::Fit3x2)),
        ((640, 480), Scale::Fit, Some(BezelArt::Fit4x3)),
        ((640, 480), Scale::Integer, Some(BezelArt::Integer4x3)),
        ((720, 480), Scale::Integer, Some(BezelArt::Default)),
        ((720, 480), Scale::FullScreen, None),
        ((640, 480), Scale::FullScreen, None),
    ] {
        let (_d, a) = playing(
            Platform::Gb,
            panel,
            SlotState {
                bezel_gb: BezelPick::Default,
                scale_gb: scale,
                ..SlotState::default()
            },
        );
        assert_eq!(a.bezel_art(), want, "{panel:?} {scale:?}");
    }
    let (_d, a) = playing(
        Platform::Gb,
        (720, 480),
        SlotState {
            bezel_gb: BezelPick::Default,
            scale_gb: Scale::FullScreen,
            ..SlotState::default()
        },
    );
    assert_eq!(a.bezel_art(), None);
}

#[test]
fn a_saved_integer_on_3_by_2_draws_the_integer_rect() {
    let (_d, a) = playing(
        Platform::Gb,
        (720, 480),
        SlotState {
            scale_gb: Scale::Integer,
            ..SlotState::default()
        },
    );
    assert_eq!((a.game_size().w, a.game_size().h), (480, 432));
    assert_eq!(a.game_offset(), (120, 24));
    assert_eq!(a.source_rect(), WHOLE_TEXTURE);
}

fn bezels_in(
    platform: Platform,
    panel: (u32, u32),
    scale: Scale,
    picked: &str,
    files: &[&str],
) -> (Vec<String>, BezelPick) {
    let pick = BezelPick::File(picked.into());
    let state = match platform {
        Platform::Gba => SlotState {
            bezel_gba: pick,
            scale_gba: scale,
            ..SlotState::default()
        },
        _ => SlotState {
            bezel_gb: pick,
            scale_gb: scale,
            ..SlotState::default()
        },
    };
    let d = root_for(platform);
    let dir = d.path().join("Bezels").join(platform.dir_name());
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::write(dir.join(f), b"").unwrap();
    }
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Game".into()),
            cart_platform: Some(platform),
            clock_set: true,
            ..state
        },
    )
    .unwrap();
    let mut a = App::boot(d.path());
    a.set_panel(panel);
    let mut shown: Vec<String> = a
        .bezel_files(platform)
        .into_iter()
        .filter(|name| {
            slot_store::bezel_scale(name).is_none_or(|s| s == a.resolved_scale(platform))
        })
        .collect();
    shown.sort();
    (shown, a.resolved_bezel(platform))
}

const ALL: [&str; 3] = ["Shell_fit.png", "Shell_integer.png", "Wood.png"];

#[test]
fn a_custom_bezel_hole_centres_the_game_rect() {
    let (d, mut a) = playing(
        Platform::Gba,
        (640, 480),
        SlotState {
            bezel_gba: BezelPick::File("stock.png".into()),
            scale_gba: Scale::Fit,
            ..SlotState::default()
        },
    );
    let dir = d.path().join("Bezels/GBA");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("stock.png"), b"").unwrap();
    a.set_bezel_hole(Some((0, 26, 640, 427)));
    assert_eq!(a.game_offset(), (0, 26));
}

#[test]
fn a_gbc_bezel_hole_centres_the_integer_rect() {
    let (d, mut a) = playing(
        Platform::Gbc,
        (640, 480),
        SlotState {
            bezel_gbc: BezelPick::File("gbc_int_ant.png".into()),
            scale_gbc: Scale::Integer,
            ..SlotState::default()
        },
    );
    let dir = d.path().join("Bezels/GBC");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("gbc_int_ant.png"), b"").unwrap();
    a.set_bezel_hole(Some((83, 27, 474, 426)));
    assert_eq!(a.game_offset(), (80, 24));
}

#[test]
fn no_bezel_centres_the_game_rect() {
    let (_d, a) = playing(
        Platform::Gba,
        (640, 480),
        SlotState {
            bezel_gba: BezelPick::None,
            scale_gba: Scale::Fit,
            ..SlotState::default()
        },
    );
    assert_eq!(a.game_offset(), (0, 26));
}

#[test]
fn the_bezel_list_holds_plain_bezels_and_those_for_the_scale() {
    for (platform, panel, scale, want) in [
        (
            Platform::Gb,
            (640, 480),
            Scale::Integer,
            ["Shell_integer.png", "Wood.png"],
        ),
        (
            Platform::Gb,
            (640, 480),
            Scale::Fit,
            ["Shell_fit.png", "Wood.png"],
        ),
        (
            Platform::Gb,
            (720, 480),
            Scale::Integer,
            ["Shell_integer.png", "Wood.png"],
        ),
        (
            Platform::Gba,
            (640, 480),
            Scale::Fit,
            ["Shell_fit.png", "Wood.png"],
        ),
    ] {
        let (shown, _) = bezels_in(platform, panel, scale, "Wood.png", &ALL);
        assert_eq!(shown, want, "{platform:?} {panel:?} {scale:?}");
    }
}

#[test]
fn a_plain_bezel_stays_picked_in_every_scale() {
    for scale in [Scale::Integer, Scale::Fit, Scale::FullScreen] {
        let (_, pick) = bezels_in(Platform::Gb, (640, 480), scale, "Wood.png", &ALL);
        assert_eq!(pick, BezelPick::File("Wood.png".into()), "{scale:?}");
    }
}

#[test]
fn a_suffixed_bezel_follows_the_scale_to_its_matching_file() {
    let (_, pick) = bezels_in(
        Platform::Gb,
        (640, 480),
        Scale::Integer,
        "Shell_fit.png",
        &ALL,
    );
    assert_eq!(pick, BezelPick::File("Shell_integer.png".into()));
    let only_fit = ["Shell_fit.png", "Wood.png"];
    let (_, pick) = bezels_in(
        Platform::Gb,
        (640, 480),
        Scale::Integer,
        "Shell_fit.png",
        &only_fit,
    );
    assert_eq!(pick, BezelPick::None);
}
