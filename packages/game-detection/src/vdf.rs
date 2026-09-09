//! A parser for Valve's KeyValues text format (`.vdf`, `.acf`).
//!
//! Steam stores the list of library folders, and one manifest per installed
//! game, in this format. Everything GameHub knows about a Steam install comes
//! through here, so the parser has to survive real files rather than tidy ones:
//! escaped quotes, `//` comments, conditional suffixes like `[$WIN32]`, CRLF,
//! a UTF-8 BOM, and blocks nested deeper than the format's own tools produce.

use std::collections::BTreeMap;

use crate::error::{DetectError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Object(Object),
}

pub type Object = BTreeMap<String, Value>;

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            Value::Object(_) => None,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(o) => Some(o),
            Value::String(_) => None,
        }
    }

    /// Case-insensitive lookup — Steam is not consistent about capitalisation
    /// between versions ("AppState" vs "appstate", "path" vs "Path").
    pub fn get(&self, key: &str) -> Option<&Value> {
        let object = self.as_object()?;
        object.get(key).or_else(|| {
            object
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v)
        })
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get_str(key)?.trim().parse().ok()
    }
}

pub fn parse(input: &str) -> Result<Value> {
    let mut parser = Parser {
        bytes: input.trim_start_matches('\u{feff}').as_bytes(),
        pos: 0,
    };
    let mut root = Object::new();
    loop {
        parser.skip_trivia();
        if parser.done() {
            break;
        }
        let (key, value) = parser.pair()?;
        merge(&mut root, key, value);
    }
    Ok(Value::Object(root))
}

/// Repeated keys are normal in KeyValues (several `"1"` entries under
/// `libraryfolders`, for instance). The later one wins, except that two objects
/// under the same key are merged, which is what Steam itself does.
fn merge(target: &mut Object, key: String, value: Value) {
    match (target.get_mut(&key), value) {
        (Some(Value::Object(existing)), Value::Object(incoming)) => {
            for (k, v) in incoming {
                merge(existing, k, v);
            }
        }
        (_, value) => {
            target.insert(key, value);
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn done(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_whitespace() => self.pos += 1,
                // Line comments. Steam writes these into libraryfolders.vdf.
                Some(b'/') if self.bytes.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(b) = self.peek() {
                        self.pos += 1;
                        if b == b'\n' {
                            break;
                        }
                    }
                }
                // Conditionals such as [$WIN32] apply to the previous token and
                // carry no data GameHub needs.
                Some(b'[') => {
                    while let Some(b) = self.peek() {
                        self.pos += 1;
                        if b == b']' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn token(&mut self) -> Result<String> {
        match self.peek() {
            Some(b'"') => self.quoted(),
            Some(_) => Ok(self.bare()),
            None => Err(DetectError::Parse {
                what: "vdf",
                detail: "unexpected end of file".into(),
            }),
        }
    }

    fn quoted(&mut self) -> Result<String> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let Some(b) = self.peek() else {
                return Err(DetectError::Parse {
                    what: "vdf",
                    detail: "unterminated string".into(),
                });
            };
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let escaped = self.peek().unwrap_or(b'\\');
                    self.pos += 1;
                    out.push(match escaped {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        other => other as char,
                    });
                }
                _ => {
                    // Multi-byte UTF-8 has to survive intact; push raw bytes and
                    // let the tail of the sequence come through the same path.
                    let start = self.pos - 1;
                    let len = utf8_len(b);
                    self.pos = start + len;
                    let slice = self.bytes.get(start..self.pos).unwrap_or(&[]);
                    out.push_str(&String::from_utf8_lossy(slice));
                }
            }
        }
    }

    fn bare(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() || b == b'"' || b == b'{' || b == b'}' {
                break;
            }
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned()
    }

    fn pair(&mut self) -> Result<(String, Value)> {
        let key = self.token()?;
        self.skip_trivia();
        match self.peek() {
            Some(b'{') => {
                self.pos += 1;
                let object = self.object()?;
                Ok((key, Value::Object(object)))
            }
            Some(_) => {
                let value = self.token()?;
                Ok((key, Value::String(value)))
            }
            None => Err(DetectError::Parse {
                what: "vdf",
                detail: format!("key \"{key}\" has no value"),
            }),
        }
    }

    fn object(&mut self) -> Result<Object> {
        let mut out = Object::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(out);
                }
                None => {
                    return Err(DetectError::Parse {
                        what: "vdf",
                        detail: "unclosed block".into(),
                    })
                }
                Some(_) => {
                    let (key, value) = self.pair()?;
                    merge(&mut out, key, value);
                }
            }
        }
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_libraryfolders_file() {
        // Trimmed from an actual libraryfolders.vdf, comments and all.
        let input = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"contentid"		"123"
		"totalsize"		"0"
		"apps"
		{
			"440"		"1234567"
			"570"		"7654321"
		}
	}
	// a second drive
	"1"
	{
		"path"		"D:\\SteamLibrary"
		"apps"
		{
			"1174180"		"99999"
		}
	}
}
"#;
        let root = parse(input).unwrap();
        let folders = root.get("libraryfolders").unwrap();
        assert_eq!(folders.get("0").unwrap().get_str("path").unwrap(), r"C:\Program Files (x86)\Steam");
        assert_eq!(folders.get("1").unwrap().get_str("path").unwrap(), r"D:\SteamLibrary");
        let apps = folders.get("1").unwrap().get("apps").unwrap().as_object().unwrap();
        assert!(apps.contains_key("1174180"));
    }

    #[test]
    fn parses_an_appmanifest() {
        let input = r#"
"AppState"
{
	"appid"		"440"
	"name"		"Team Fortress 2"
	"StateFlags"		"4"
	"installdir"		"Team Fortress 2"
	"SizeOnDisk"		"24696061952"
	"LastPlayed"		"1735689600"
}
"#;
        let root = parse(input).unwrap();
        let state = root.get("AppState").unwrap();
        assert_eq!(state.get_str("name").unwrap(), "Team Fortress 2");
        assert_eq!(state.get_u64("SizeOnDisk").unwrap(), 24_696_061_952);
        // Case-insensitive, because Steam is not consistent between versions.
        assert_eq!(state.get_str("InstallDir").unwrap(), "Team Fortress 2");
    }

    #[test]
    fn survives_bom_crlf_escapes_and_conditionals() {
        let input = "\u{feff}\"root\"\r\n{\r\n\t\"quote\" \"say \\\"hi\\\"\" [$WIN32]\r\n\t\"unicode\" \"Pokémon — Zǔ\"\r\n}\r\n";
        let root = parse(input).unwrap();
        let r = root.get("root").unwrap();
        assert_eq!(r.get_str("quote").unwrap(), "say \"hi\"");
        assert_eq!(r.get_str("unicode").unwrap(), "Pokémon — Zǔ");
    }

    #[test]
    fn repeated_object_keys_merge_rather_than_replace() {
        let input = r#"
"a" { "x" "1" }
"a" { "y" "2" }
"#;
        let root = parse(input).unwrap();
        let a = root.get("a").unwrap();
        assert_eq!(a.get_str("x"), Some("1"));
        assert_eq!(a.get_str("y"), Some("2"));
    }

    #[test]
    fn rejects_truncated_files_instead_of_guessing() {
        assert!(parse("\"AppState\" {\n \"appid\" \"440\"\n").is_err());
        assert!(parse("\"key\" \"unterminated").is_err());
    }

    #[test]
    fn an_empty_file_is_an_empty_object_not_an_error() {
        let root = parse("   \n// nothing here\n").unwrap();
        assert!(root.as_object().unwrap().is_empty());
    }
}
