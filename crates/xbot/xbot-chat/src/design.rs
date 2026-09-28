//! CruxOS design checks for the embedded UI: token values, WCAG 2.x contrast,
//! and the green / glass / icon rules shared with the other X apps.

use crate::assets::{APP_JS, CRUX_CSS, INDEX, PHOSPHOR_CSS, STYLE_CSS};
use std::collections::HashMap;

fn tokens(block_start: &str) -> HashMap<String, String> {
    let start = CRUX_CSS.find(block_start).expect("token block exists");
    let end = start + CRUX_CSS[start..].find("\n}").expect("token block closes");
    CRUX_CSS[start..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("--crux-")?;
            let (name, value) = line.split_once(':')?;
            let hex = value.split_whitespace().next()?.trim_end_matches(';');
            (hex.len() == 7 && hex.starts_with('#'))
                .then(|| (name.to_string(), hex.to_ascii_lowercase()))
        })
        .collect()
}

fn light() -> HashMap<String, String> {
    tokens(":root {")
}

fn dark() -> HashMap<String, String> {
    let mut all = light();
    all.extend(tokens("html[data-theme=\"dark\"] {"));
    all
}

fn luminance(hex: &str) -> f64 {
    let channel = |i: usize| {
        let c = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap()) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[test]
fn tokens_match_the_cruxos_palette() {
    let expected_light = [
        ("bg", "#ffffff"),
        ("surface", "#f5f5f7"),
        ("elevated", "#e8e8ed"),
        ("border", "#d2d2d7"),
        ("text", "#1d1d1f"),
        ("text-secondary", "#6e6e73"),
        ("accent", "#a1ed72"),
        ("on-accent", "#0a0a0b"),
        ("accent-text", "#37770f"),
        ("selected", "#e5fad8"),
        ("destructive", "#d70015"),
    ];
    let expected_dark = [
        ("bg", "#0a0a0b"),
        ("surface", "#161618"),
        ("elevated", "#1f1f22"),
        ("border", "#2c2c30"),
        ("text", "#f5f5f7"),
        ("text-secondary", "#a1a1a6"),
        ("accent-text", "#a1ed72"),
        ("selected", "#2b3c22"),
        ("destructive", "#ff453a"),
    ];
    let (light, dark) = (light(), dark());
    for (name, hex) in expected_light {
        assert_eq!(light[name], hex, "light --crux-{name}");
    }
    for (name, hex) in expected_dark {
        assert_eq!(dark[name], hex, "dark --crux-{name}");
    }
}

#[test]
fn token_pairs_meet_wcag_contrast() {
    for t in [light(), dark()] {
        for surface in ["bg", "surface", "elevated"] {
            assert!(
                contrast(&t["text"], &t[surface]) >= 7.0,
                "text on {surface}"
            );
            assert!(
                contrast(&t["accent-text"], &t[surface]) >= 4.5,
                "accent text on {surface}"
            );
        }
        for surface in ["bg", "surface"] {
            assert!(contrast(&t["text-secondary"], &t[surface]) >= 4.5);
        }
        for fill in ["accent", "accent-hover", "accent-pressed"] {
            assert!(
                contrast(&t["on-accent"], &t[fill]) >= 7.0,
                "glyph on {fill}"
            );
        }
        for row in ["selected", "selected-surface"] {
            assert!(
                contrast(&t["selected-text"], &t[row]) >= 7.0,
                "text on {row}"
            );
        }
        assert!(contrast(&t["destructive"], &t["bg"]) >= 4.5);
    }
    assert!(
        contrast("#37770f", &light()["bg"]) >= 3.0,
        "light focus edge"
    );
    assert!(
        contrast("#a1ed72", &dark()["surface"]) >= 3.0,
        "dark focus ring"
    );
}

#[test]
fn green_is_never_text() {
    let css = format!("{CRUX_CSS}\n{STYLE_CSS}");
    for line in css.lines().map(str::trim) {
        let is_text_colour = line.starts_with("color:");
        assert!(
            !(is_text_colour && (line.contains("var(--crux-accent)") || line.contains("#a1ed72"))),
            "green used as text: {line}"
        );
    }
}

#[test]
fn window_is_opaque_and_quiet() {
    let css = format!("{CRUX_CSS}\n{STYLE_CSS}");
    assert!(!css.contains("backdrop-filter"), "app windows are opaque");
    assert!(!css.contains("infinite"), "no looping animations");
    assert!(CRUX_CSS.contains("prefers-reduced-motion"));
    assert!(CRUX_CSS.contains(":focus-visible"));
    let markup = format!("{INDEX}\n{APP_JS}");
    assert!(
        !markup.contains("material-symbols"),
        "UI glyphs are Phosphor"
    );
}

#[test]
fn every_icon_exists_in_phosphor() {
    let markup = format!("{INDEX}\n{APP_JS}");
    let mut used = 0;
    for (start, _) in markup.match_indices("ph ph-") {
        let name: String = markup[start + 6..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if name.is_empty() {
            continue; // built from a variable, e.g. `ph ph-${name}`
        }
        assert!(
            PHOSPHOR_CSS.contains(&format!(".ph.ph-{name}:before")),
            "unknown Phosphor icon {name}"
        );
        used += 1;
    }
    assert!(used >= 10);
    assert!(
        !markup.contains("class=\"icon\"") && !markup.contains("class=\"icon "),
        "icons use codepoint classes, not ligatures"
    );
}
