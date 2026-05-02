//! Parser for the top-level entities.xml registry file.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
use std::path::Path;

/// Parse entities.xml and return the list of entity type names.
///
/// The file format is:
/// ```xml
/// <root>
///     <SGWPlayer/>
///     <Account/>
///     ...
/// </root>
/// ```
/// Each self-closing child tag of <root> is an entity type name.
pub fn parse_entities_xml(path: &Path) -> Vec<String> {
    let xml = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            println!(
                "cargo:warning=Could not read entities.xml ({}): {}. Generating empty registry.",
                path.display(),
                e
            );
            return Vec::new();
        }
    };

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut names = Vec::new();
    let mut depth: u32 = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                // Depth 1 = inside <root>. Self-closing tags at depth 1 are
                // entity types, but Start + End pairs at depth 1 could also be
                // entity types (the XML may use either style).
                if depth == 1 {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag != "root" {
                        names.push(tag);
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                // Self-closing tags inside <root>.
                if depth == 0 || (depth == 1 && String::from_utf8_lossy(e.name().as_ref()) != "root") {
                    // depth == 0 means we haven't seen <root> yet, which
                    // shouldn't happen, but handle gracefully. depth == 1
                    // means we're inside <root>.
                    if depth >= 1 {
                        let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        names.push(tag);
                    }
                }
            }
            Ok(Event::End(_)) => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                println!(
                    "cargo:warning=XML parse error in entities.xml: {}. Returning partial list.",
                    e
                );
                break;
            }
            _ => {}
        }
    }

    names
}
