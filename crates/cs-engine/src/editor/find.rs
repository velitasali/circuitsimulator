//! Find / replace over a buffer. C++ `FindReplace`.

use regex::{Regex, RegexBuilder};

#[derive(Clone, Debug, Default)]
pub struct FindOpts {
    pub text: String,
    pub case_sensitive: bool,
    pub whole_words: bool,
    pub regexp: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
}

fn compile(opts: &FindOpts) -> Result<Regex, String> {
    if opts.text.is_empty() {
        return Err("empty".into());
    }
    let mut pat = if opts.regexp {
        opts.text.clone()
    } else {
        regex::escape(&opts.text)
    };
    if opts.whole_words {
        pat = format!(r"\b{pat}\b");
    }
    RegexBuilder::new(&pat)
        .case_insensitive(!opts.case_sensitive)
        .build()
        .map_err(|e| e.to_string())
}

pub fn find_all(hay: &str, opts: &FindOpts) -> Result<Vec<Match>, String> {
    let re = compile(opts)?;
    Ok(re
        .find_iter(hay)
        .map(|m| Match {
            start: m.start(),
            end: m.end(),
        })
        .collect())
}

pub fn find_next(
    hay: &str,
    from: usize,
    opts: &FindOpts,
    forward: bool,
) -> Result<Option<Match>, String> {
    let all = find_all(hay, opts)?;
    if all.is_empty() {
        return Ok(None);
    }
    if forward {
        Ok(all
            .iter()
            .copied()
            .find(|m| m.start >= from)
            .or_else(|| all.first().copied()))
    } else {
        Ok(all
            .iter()
            .copied()
            .rev()
            .find(|m| m.start < from)
            .or_else(|| all.last().copied()))
    }
}

/// Literal replacement of every match (C++ `insertText`, not regex `$1`).
pub fn replace_all(hay: &str, opts: &FindOpts, repl: &str) -> Result<(String, usize), String> {
    let all = find_all(hay, opts)?;
    if all.is_empty() {
        return Ok((hay.to_string(), 0));
    }
    let mut out = String::with_capacity(hay.len());
    let mut last = 0;
    for m in &all {
        out.push_str(&hay[last..m.start]);
        out.push_str(repl);
        last = m.end;
    }
    out.push_str(&hay[last..]);
    Ok((out, all.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wraps() {
        let hay = "foo bar foo";
        let opts = FindOpts {
            text: "foo".into(),
            ..Default::default()
        };
        let m = find_next(hay, 0, &opts, true).unwrap().unwrap();
        assert_eq!(m, Match { start: 0, end: 3 });
        let m = find_next(hay, 3, &opts, true).unwrap().unwrap();
        assert_eq!(m, Match { start: 8, end: 11 });
        let m = find_next(hay, 11, &opts, true).unwrap().unwrap();
        assert_eq!(m.start, 0);
        let m = find_next(hay, 8, &opts, false).unwrap().unwrap();
        assert_eq!(m.start, 0);
    }

    #[test]
    fn whole_word_and_case() {
        let hay = "Int integer INT";
        let opts = FindOpts {
            text: "int".into(),
            whole_words: true,
            case_sensitive: false,
            regexp: false,
        };
        let all = find_all(hay, &opts).unwrap();
        assert_eq!(all.len(), 2, "{all:?}");
        assert_eq!(all[0].start, 0);
        assert_eq!(all[1].start, 12);
    }

    #[test]
    fn replace_literal() {
        let hay = "a a a";
        let opts = FindOpts {
            text: "a".into(),
            ..Default::default()
        };
        let (out, n) = replace_all(hay, &opts, "bb").unwrap();
        assert_eq!(n, 3);
        assert_eq!(out, "bb bb bb");
    }
}
