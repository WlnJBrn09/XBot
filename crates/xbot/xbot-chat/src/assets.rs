//! The chat UI, embedded in the binary and served at `xbot://localhost/`.

pub struct Asset {
    pub body: &'static [u8],
    pub mime: &'static str,
}

const fn asset(body: &'static [u8], mime: &'static str) -> Asset {
    Asset { body, mime }
}

pub const INDEX: &str = include_str!("../ui/index.html");
pub const CRUX_CSS: &str = include_str!("../ui/crux.css");
pub const PHOSPHOR_CSS: &str = include_str!("../ui/assets/phosphor.css");
pub const STYLE_CSS: &str = include_str!("../ui/style.css");
pub const APP_JS: &str = include_str!("../ui/app.js");
pub const WINDOW_ICON_PNG: &[u8] = include_bytes!("../ui/assets/icon-256.png");

pub fn lookup(path: &str) -> Option<Asset> {
    Some(match path {
        "/" | "/index.html" => asset(INDEX.as_bytes(), "text/html; charset=utf-8"),
        "/assets/phosphor.css" => asset(PHOSPHOR_CSS.as_bytes(), "text/css; charset=utf-8"),
        "/crux.css" => asset(CRUX_CSS.as_bytes(), "text/css; charset=utf-8"),
        "/style.css" => asset(STYLE_CSS.as_bytes(), "text/css; charset=utf-8"),
        "/app.js" => asset(APP_JS.as_bytes(), "text/javascript; charset=utf-8"),
        "/assets/logo.svg" => asset(include_bytes!("../ui/assets/logo.svg"), "image/svg+xml"),
        "/assets/InterVariable.woff2" => asset(
            include_bytes!("../ui/assets/InterVariable.woff2"),
            "font/woff2",
        ),
        "/assets/Phosphor.woff2" => {
            asset(include_bytes!("../ui/assets/Phosphor.woff2"), "font/woff2")
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_local_reference_is_served() {
        // Each source with the directory its relative URLs resolve against.
        let sources = [
            (INDEX, "/"),
            (CRUX_CSS, "/"),
            (STYLE_CSS, "/"),
            (PHOSPHOR_CSS, "/assets/"),
        ];
        let mut checked = 0;
        for (source, base) in sources {
            for part in source.split(['"', '(', ')']) {
                let is_local = part.ends_with(".css")
                    || part.ends_with(".js")
                    || part.ends_with(".svg")
                    || part.ends_with(".woff2");
                if is_local && !part.contains(' ') && !part.contains(':') {
                    let path = format!("{base}{}", part.trim_start_matches("./"));
                    assert!(
                        lookup(&path).is_some(),
                        "{path} is referenced but not served"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked >= 6);
    }

    #[test]
    fn unknown_paths_are_not_served() {
        assert!(lookup("/../Cargo.toml").is_none());
        assert!(lookup("/api/status").is_none());
    }
}
