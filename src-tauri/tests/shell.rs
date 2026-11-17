//! Shell tests. They live here, not in the library, because only test targets can be given
//! the Windows manifest Tauri needs to start (see build.rs).

use lovelydl_lib::badge;

/// Regenerates `ui/src/ipc/bindings.ts`. CI fails if the committed file then differs.
#[test]
fn bindings_are_exported() {
    let path = lovelydl_lib::export_bindings().unwrap_or_else(|e| panic!("{e}"));
    let text = std::fs::read_to_string(path).unwrap_or_default();
    assert!(text.contains("getSnapshot") && text.contains("export type Change"), "bindings look incomplete");
}

fn white_pixels(px: &[u8]) -> usize {
    px.chunks(4).filter(|p| p == &[0xFF, 0xFF, 0xFF, 0xFF]).count()
}

#[test]
fn badge_digits_are_drawn_inside_the_disc() {
    let side = badge::size() as usize;
    for count in [0, 1, 7, 10, 42, 99, 250] {
        let px = badge::render(count);
        assert_eq!(px.len(), side * side * 4);
        assert!(white_pixels(&px) > 20, "{count} draws something");
        // Corners stay transparent.
        assert_eq!(px[3], 0);
        assert_eq!(px[px.len() - 1], 0);
    }
    assert_eq!(badge::render(250), badge::render(99), "large counts show 99");
    assert_ne!(badge::render(1), badge::render(7));
}

#[cfg(windows)]
#[test]
fn reads_this_machine() {
    use lovelydl_lib::platform;
    assert!(platform::build_number() > 10000, "a real Windows build number");
    if let Some(accent) = platform::accent_color() {
        assert!(accent.len() == 7 && accent.starts_with('#'), "{accent}");
    }
    let _ = platform::clipboard_sequence();
}
