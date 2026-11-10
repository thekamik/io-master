//! The values of a report made harmless in the language of its file (found
//! 2026-11-10): a value is a text of the evaluation (the findings the AI
//! wrote, the name of a document of the manufacturer), never a part of the
//! template, so in a report of HTML or of LaTeX it must stay text. Without
//! it a value could bring a script into a page, or a command into a document
//! that is compiled.
//!
//! ```text
//! ReportTemplate::extension ──► escape_of ──► Escape ──► escaped(value) ──► the value as the report writes it
//!   html, htm, xhtml, xml   Escape::Markup   & < > " '  as entities
//!   tex, latex              Escape::Latex    \ { } $ & # _ % ~ ^  as their commands
//!   any other (md, txt)     Escape::None     the value as it is
//! ```
//!
//! Markdown is written as it is: the texts of an evaluation may be written
//! in Markdown themselves, and a file of Markdown runs nothing.

/// How the values of a report are written into its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// As they are (Markdown, plain text).
    None,
    /// As the text of HTML or XML.
    Markup,
    /// As the text of LaTeX.
    Latex,
}

/// How a report of the extension `extension` writes its values.
///
/// Input: `extension`, the extension of a stored template, in lower case
/// without its dot (`"html"`).
/// Output: the way (`Escape::Markup`); `Escape::None` for an extension that
/// is neither of HTML or XML nor of LaTeX.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `filled_report`.
pub fn escape_of(extension: &str) -> Escape {
    match extension {
        "html" | "htm" | "xhtml" | "xml" => Escape::Markup,
        "tex" | "latex" => Escape::Latex,
        _ => Escape::None,
    }
}

/// What stands for the character `character` in the text of HTML or XML.
///
/// Called by: [`escaped`].
fn markup_of(character: char) -> Option<&'static str> {
    match character {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        '\'' => Some("&#39;"),
        _ => None,
    }
}

/// What stands for the character `character` in the text of LaTeX.
///
/// Called by: [`escaped`].
fn latex_of(character: char) -> Option<&'static str> {
    match character {
        '\\' => Some("\\textbackslash{}"),
        '{' => Some("\\{"),
        '}' => Some("\\}"),
        '$' => Some("\\$"),
        '&' => Some("\\&"),
        '#' => Some("\\#"),
        '_' => Some("\\_"),
        '%' => Some("\\%"),
        '~' => Some("\\textasciitilde{}"),
        '^' => Some("\\textasciicircum{}"),
        _ => None,
    }
}

/// The value `value` as a report written by `escape` holds it: every
/// character its language reads as more than text replaced.
///
/// Input: `value`, the value of a field of the evaluation, bounded by its
/// store; `escape`, from [`escape_of`].
/// Output: the value (`"a &lt; b"` of "a < b" in HTML); the same value for
/// `Escape::None`.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `values_of`.
/// Calls: [`markup_of`], [`latex_of`].
pub fn escaped(value: String, escape: Escape) -> String {
    if escape == Escape::None {
        return value;
    }

    let mut written = String::with_capacity(value.len());

    for character in value.chars() {
        let replacement = match escape {
            Escape::Markup => markup_of(character),
            Escape::Latex => latex_of(character),
            Escape::None => None,
        };

        match replacement {
            Some(text) => written.push_str(text),
            None => written.push(character),
        }
    }

    written
}

#[cfg(test)]
mod tests {
    //! Tests of the values of a report: harmless in HTML and in LaTeX, as
    //! they are elsewhere.

    use super::*;

    #[test]
    fn a_value_stays_text_in_the_language_of_its_report() {
        let value = "<script>alert('x')</script> & \"more\"";

        assert_eq!(
            escaped(value.to_owned(), escape_of("html")),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; &quot;more&quot;"
        );
        assert_eq!(
            escaped("\\input{/etc/passwd} 50% of $x_1^2 ~ #1 & y".to_owned(), escape_of("tex")),
            "\\textbackslash{}input\\{/etc/passwd\\} 50\\% of \\$x\\_1\\textasciicircum{}2 \\textasciitilde{} \\#1 \\& y"
        );
        assert_eq!(escaped(value.to_owned(), escape_of("md")), value);
        assert_eq!(escape_of("txt"), Escape::None);
    }
}
