use std::sync::OnceLock;

use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{ClassStyle, css_for_theme_with_class_style, line_tokens_to_classed_spans};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

const THEME_NAME: &str = "InspiredGitHub";

/// Class style shared by blob highlighting and the comrak syntect plugin,
/// so both are covered by theme_css().
pub const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "syn-" };

pub fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(|| {
        let _timer = crate::debug_timing::start("highlight.syntax_init");
        SyntaxSet::load_defaults_newlines()
    })
}

fn theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let _timer = crate::debug_timing::start("highlight.theme_init");
        ThemeSet::load_defaults().themes[THEME_NAME].clone()
    })
}

/// CSS rules for all syntect classes used in rendered pages; appended to the served stylesheet.
pub fn theme_css() -> &'static str {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(|| {
        let _timer = crate::debug_timing::start("highlight.css_init");
        css_for_theme_with_class_style(theme(), CLASS_STYLE).expect("default theme is valid")
    })
}

pub fn comrak_adapter() -> &'static comrak::plugins::syntect::SyntectAdapter {
    static ADAPTER: OnceLock<comrak::plugins::syntect::SyntectAdapter> = OnceLock::new();
    ADAPTER.get_or_init(|| {
        let _timer = crate::debug_timing::start("highlight.markdown_init");
        comrak::plugins::syntect::SyntectAdapterBuilder::new()
            .css_with_class_prefix("syn-")
            .build()
    })
}

/// Whether a blob of `size` bytes should be highlighted under the configured
/// `limit`: 0 disables highlighting, negative values always highlight.
pub fn within_size_limit(size: usize, limit: i64) -> bool {
    limit < 0 || (limit > 0 && size <= limit as usize)
}

/// Highlight source code as per-line HTML fragments using CSS classes.
/// Returns None for unknown or plain-text file types so callers fall back to escaped text.
pub fn highlight_lines(path: &str, lines: &[String]) -> Option<Vec<String>> {
    let set = syntax_set();
    let name = path.rsplit('/').next().unwrap_or(path);
    let syntax = set.find_syntax_by_extension(name).or_else(|| {
        name.rsplit_once('.')
            .and_then(|(_, ext)| set.find_syntax_by_extension(ext))
    })?;
    if syntax.name == "Plain Text" {
        return None;
    }
    // Same pipeline as syntect's ClassedHTMLGenerator, but keeping per-line fragments
    // so the caller can render its own line-number table.
    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let mut text = String::with_capacity(line.len() + 1);
        text.push_str(line);
        text.push('\n');
        let ops = parse_state.parse_line(&text, set).ok()?;
        let (html, _delta) =
            line_tokens_to_classed_spans(&text, ops.as_slice(), CLASS_STYLE, &mut scope_stack)
                .ok()?;
        out.push(html);
    }
    Some(out)
}
