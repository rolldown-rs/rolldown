use std::collections::HashSet;

use once_cell::sync::Lazy;

static RESERVED_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
  "break case class catch const continue debugger default delete do else export extends finally for function if import in instanceof let new return super switch this throw try typeof var void while with yield enum await implements package protected static interface private public".split(' ').collect()
});

static BUILTINS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
  "Infinity NaN undefined null true false eval uneval isFinite isNaN parseFloat parseInt decodeURI decodeURIComponent encodeURI encodeURIComponent escape unescape Object Function Boolean Symbol Error EvalError InternalError RangeError ReferenceError SyntaxError TypeError URIError Number Math Date String RegExp Array Int8Array Uint8Array Uint8ClampedArray Int16Array Uint16Array Int32Array Uint32Array Float32Array Float64Array Map Set WeakMap WeakSet SIMD ArrayBuffer DataView JSON Promise Generator GeneratorFunction Reflect Proxy Intl".split(' ').collect()
});

static BLACKLISTED: Lazy<HashSet<&'static str>> = Lazy::new(|| {
  BUILTINS
    .clone()
    .into_iter()
    .chain(RESERVED_WORDS.clone().into_iter())
    .collect()
});

static ILLEGAL_CHARACTERS_RE: Lazy<regex::Regex> =
  Lazy::new(|| regex::Regex::new(r"[^$_a-zA-Z0-9]").unwrap());

static DIGITS_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\d").unwrap());
#[inline]
fn starts_with_digit(s: &str) -> bool {
  DIGITS_RE.is_match(&s[0..1])
}

#[inline]
pub fn is_legal(s: &str) -> bool {
  if starts_with_digit(s) || BLACKLISTED.contains(s) {
    false
  } else {
    !ILLEGAL_CHARACTERS_RE.is_match(s)
  }
}

pub static UN_LEGAL_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"-(\w)").unwrap());

pub fn make_legal(s: &str) -> String {
  // 	str = str.replace(/-(\w)/g, (_, letter) => letter.toUpperCase()).replace(ILLEGAL_CHARACTERS, '_');
  let mut s = s.to_string();
  if starts_with_digit(&s) || BLACKLISTED.contains(s.as_str()) {
    s.insert(0, '_');
  }
  s
}
