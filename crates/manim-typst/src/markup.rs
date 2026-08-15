//! Authoring-time Pango markup → Typst markup.
//!
//! ManimCE `MarkupText` is Pango. We compile Typst in-process, so a small
//! tag subset is rewritten here — never in the per-frame path.

/// Convert a Pango markup subset to Typst. Strings without `<` or `&` are
/// returned unchanged so native Typst (`*bold*`) still works.
pub fn pango_to_typst(input: &str) -> String {
    if !input.contains('<') && !input.contains('&') {
        return input.to_string();
    }
    convert_markup(input)
}

fn convert_markup(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while !rest.is_empty() {
        if rest.starts_with('<') {
            if let Some((tag, after)) = parse_tag(rest) {
                match tag {
                    Tag::Close(_) => {
                        rest = after;
                    }
                    Tag::Open {
                        name,
                        attrs,
                        self_closing,
                    } => {
                        if self_closing || name.eq_ignore_ascii_case("br") {
                            out.push_str("#linebreak()");
                            rest = after;
                            continue;
                        }
                        match take_inner(after, &name) {
                            Some((inner, tail)) => {
                                out.push_str(&wrap_tag(&name, &attrs, &convert_markup(inner)));
                                rest = tail;
                            }
                            None => {
                                out.push_str(&escape_typst_plain("<"));
                                rest = &rest[1..];
                            }
                        }
                    }
                }
            } else {
                out.push_str(&escape_typst_plain("<"));
                rest = &rest[1..];
            }
        } else if rest.starts_with('&') {
            if let Some((ch, after)) = decode_entity(rest) {
                out.push_str(&escape_typst_plain(&ch));
                rest = after;
            } else {
                out.push_str(&escape_typst_plain("&"));
                rest = &rest[1..];
            }
        } else {
            let end = rest.find(['<', '&']).unwrap_or(rest.len());
            out.push_str(&escape_typst_plain(&rest[..end]));
            rest = &rest[end..];
        }
    }
    out
}

#[derive(Debug)]
enum Tag {
    Open {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    Close(String),
}

fn parse_tag(s: &str) -> Option<(Tag, &str)> {
    if !s.starts_with('<') {
        return None;
    }
    let rest = &s[1..];
    let close = rest.find('>')?;
    let inside = rest[..close].trim();
    let after = &rest[close + 1..];
    if inside.is_empty() {
        return None;
    }
    if let Some(name) = inside.strip_prefix('/') {
        return Some((Tag::Close(name.trim().to_string()), after));
    }
    let self_closing = inside.ends_with('/');
    let inside = inside.trim_end_matches('/').trim();
    let (name, attrs_src) = split_name_attrs(inside);
    if name.is_empty() {
        return None;
    }
    Some((
        Tag::Open {
            name,
            attrs: parse_attrs(attrs_src),
            self_closing,
        },
        after,
    ))
}

fn split_name_attrs(inside: &str) -> (String, &str) {
    let end = inside
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(inside.len());
    (inside[..end].to_string(), inside[end..].trim())
}

fn parse_attrs(mut src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    while !src.is_empty() {
        let eq = match src.find('=') {
            Some(i) => i,
            None => break,
        };
        let key = src[..eq].trim().to_string();
        if key.is_empty() {
            break;
        }
        src = src[eq + 1..].trim_start();
        let (val, rest) = if let Some(q) = src.chars().next().filter(|c| *c == '"' || *c == '\'') {
            let body = &src[1..];
            match body.find(q) {
                Some(end) => (&body[..end], &body[end + 1..]),
                None => (body, ""),
            }
        } else {
            let end = src.find(char::is_whitespace).unwrap_or(src.len());
            (&src[..end], &src[end..])
        };
        out.push((key, val.to_string()));
        src = rest.trim_start();
    }
    out
}

fn take_inner<'a>(rest: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 1i32;
    let mut i = 0usize;
    while i < rest.len() {
        if rest[i..].starts_with('<') {
            if let Some((tag, after)) = parse_tag(&rest[i..]) {
                match &tag {
                    Tag::Open { name: n, .. } if n.eq_ignore_ascii_case(name) => depth += 1,
                    Tag::Close(n) if n.eq_ignore_ascii_case(name) => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((&rest[..i], after));
                        }
                    }
                    _ => {}
                }
                i = rest.len() - after.len();
                continue;
            }
        }
        i += rest[i..].chars().next()?.len_utf8();
    }
    None
}

fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn wrap_tag(name: &str, attrs: &[(String, String)], inner: &str) -> String {
    let n = name.to_ascii_lowercase();
    match n.as_str() {
        "b" | "strong" => format!("#strong[{inner}]"),
        "i" | "em" => format!("#emph[{inner}]"),
        "u" => format!("#underline[{inner}]"),
        "s" | "strike" | "del" => format!("#strike[{inner}]"),
        "tt" | "code" => format!("#text(font: \"DejaVu Sans Mono\")[{inner}]"),
        "big" => format!("#text(size: 1.2em)[{inner}]"),
        "small" => format!("#text(size: 0.8em)[{inner}]"),
        "sup" => format!("#super[{inner}]"),
        "sub" => format!("#sub[{inner}]"),
        "span" => wrap_span(attrs, inner),
        "gradient" => {
            let from = attr(attrs, "from").unwrap_or("white");
            format!("#text(fill: {})[{inner}]", color_to_typst(from))
        }
        _ => inner.to_string(),
    }
}

fn wrap_span(attrs: &[(String, String)], inner: &str) -> String {
    let mut args = Vec::new();
    let mut body = inner.to_string();
    if let Some(c) = attr(attrs, "foreground")
        .or_else(|| attr(attrs, "fgcolor"))
        .or_else(|| attr(attrs, "color"))
    {
        args.push(format!("fill: {}", color_to_typst(c)));
    }
    if let Some(sz) = attr(attrs, "size") {
        args.push(format!("size: {}", pango_size_to_typst(sz)));
    }
    if let Some(font) = attr(attrs, "font_family").or_else(|| attr(attrs, "face")) {
        args.push(format!("font: \"{}\"", font.replace('"', "")));
    }
    if attr(attrs, "underline").is_some() {
        body = format!("#underline[{body}]");
    }
    if attr(attrs, "strikethrough").is_some_and(|v| v.eq_ignore_ascii_case("true")) {
        body = format!("#strike[{body}]");
    }
    if args.is_empty() {
        body
    } else {
        format!("#text({})[{body}]", args.join(", "))
    }
}

fn pango_size_to_typst(size: &str) -> String {
    let s = size.trim();
    match s.to_ascii_lowercase().as_str() {
        "xx-small" => "0.6em".into(),
        "x-small" => "0.75em".into(),
        "small" => "0.85em".into(),
        "medium" => "1em".into(),
        "large" => "1.2em".into(),
        "x-large" => "1.4em".into(),
        "xx-large" => "1.7em".into(),
        other => {
            if let Some(n) = other.strip_suffix("pt") {
                format!("{}pt", n.trim())
            } else if other.chars().all(|c| c.is_ascii_digit()) {
                // Pango absolute sizes are 1024ths of a point.
                let n: f64 = other.parse().unwrap_or(0.0);
                if n > 64.0 {
                    format!("{:.1}pt", n / 1024.0)
                } else {
                    format!("{other}pt")
                }
            } else {
                "1em".into()
            }
        }
    }
}

fn color_to_typst(s: &str) -> String {
    let t = s.trim();
    if t.starts_with('#') {
        return format!("rgb(\"{t}\")");
    }
    if let Some(c) = manim_core::palette::named(t) {
        let r = c.to_rgba8();
        return format!("rgb({}, {}, {})", r.r, r.g, r.b);
    }
    t.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn escape_typst_plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '#' | '$' | '*' | '_' | '`' | '@' | '\\' | '[' | ']' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn decode_entity(s: &str) -> Option<(String, &str)> {
    if !s.starts_with('&') {
        return None;
    }
    let end = s.find(';')?;
    let body = &s[1..end];
    let after = &s[end + 1..];
    let ch = match body {
        "lt" => "<".into(),
        "gt" => ">".into(),
        "amp" => "&".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        "nbsp" => " ".into(),
        rest if rest.starts_with('#') => {
            let n = if rest.len() > 2 && (rest.as_bytes()[1] == b'x' || rest.as_bytes()[1] == b'X')
            {
                u32::from_str_radix(&rest[2..], 16).ok()?
            } else {
                rest[1..].parse().ok()?
            };
            char::from_u32(n)?.to_string()
        }
        _ => return None,
    };
    Some((ch, after))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typst_passthrough_without_tags() {
        assert_eq!(pango_to_typst("*bold* _i_"), "*bold* _i_");
    }

    #[test]
    fn bold_italic_and_color_span() {
        let t = pango_to_typst("<b>Bold</b> and <i>italic</i>");
        assert!(t.contains("#strong[Bold]"), "{t}");
        assert!(t.contains("#emph[italic]"), "{t}");
        let c = pango_to_typst(r#"<span foreground="blue">Blue</span>"#);
        assert!(c.contains("fill:"), "{c}");
        assert!(c.contains("Blue"), "{c}");
    }

    #[test]
    fn entities_decode() {
        assert_eq!(pango_to_typst("A &amp; B"), "A & B");
        assert!(pango_to_typst("&#169;").contains("©"));
    }
}
