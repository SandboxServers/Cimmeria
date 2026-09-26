//! SmartFoxServer 1.x XML protocol codec.
//!
//! The minigame Flash SWFs communicate via null-terminated XML strings over TCP
//! using the SmartFoxServer message format. This module handles parsing incoming
//! messages and encoding outgoing messages.
//!
//! Reference: `src/baseapp/minigame_connection.cpp`

use std::collections::HashMap;

// ── Value types ──────────────────────────────────────────────────────────────

/// A SmartFoxServer variable value.
#[derive(Debug, Clone)]
pub enum SfsValue {
    String(String),
    Number(f64),
    Bool(bool),
    Object(HashMap<String, SfsValue>),
}

impl SfsValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SfsValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            SfsValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        self.as_f64().map(|n| n as i32)
    }
}

// ── Parsed message types ─────────────────────────────────────────────────────

/// A parsed incoming SFS message.
#[derive(Debug)]
pub enum SfsMessage {
    /// `<msg t='sys'><body action='verChk'><ver v='N'/></body></msg>`
    VersionCheck { version: u32 },
    /// `<msg t='sys'><body action='login'><login z='zone'><nick>...</nick><pword>...</pword></login></body></msg>`
    Login {
        zone: String,
        nick: String,
        password: String,
    },
    /// `<msg t='xt'><body action='xtReq'><![CDATA[<dataObj>...</dataObj>]]></body></msg>`
    ExtensionRequest {
        cmd: String,
        params: HashMap<String, SfsValue>,
    },
}

// ── Parsing ──────────────────────────────────────────────────────────────────

/// Parse a raw XML message string into an `SfsMessage`.
pub fn parse_message(xml: &str) -> Option<SfsMessage> {
    // Use quick-xml for parsing
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);

    // Find <msg t='...'>
    let mut msg_type = String::new();
    let mut in_body = false;
    let mut body_action = String::new();
    let mut body_r = 0i32;

    // For verChk
    let mut ver_version: Option<u32> = None;

    // For login
    let mut login_zone = String::new();
    let mut in_nick = false;
    let mut in_pword = false;
    let mut nick = String::new();
    let mut password = String::new();

    // For xtReq — the body text is CDATA containing XML
    let mut body_text = String::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                match name {
                    "msg" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"t" {
                                msg_type = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "body" => {
                        in_body = true;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"action" => {
                                    body_action = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"r" => {
                                    body_r =
                                        String::from_utf8_lossy(&attr.value).parse().unwrap_or(0);
                                }
                                _ => {}
                            }
                        }
                    }
                    "ver" if in_body => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"v" {
                                ver_version = String::from_utf8_lossy(&attr.value).parse().ok();
                            }
                        }
                    }
                    "login" if in_body => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"z" {
                                login_zone = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "nick" => in_nick = true,
                    "pword" => in_pword = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().unwrap_or_default().to_string();
                if in_nick {
                    nick = text;
                } else if in_pword {
                    password = text;
                } else if in_body {
                    body_text.push_str(&text);
                }
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if in_nick {
                    nick = text;
                } else if in_pword {
                    password = text;
                } else if in_body {
                    body_text.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                match name {
                    "nick" => in_nick = false,
                    "pword" => in_pword = false,
                    "body" => in_body = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }

    let _ = body_r; // Used in routing but not needed for parsing

    match (msg_type.as_str(), body_action.as_str()) {
        ("sys", "verChk") => Some(SfsMessage::VersionCheck {
            version: ver_version.unwrap_or(0),
        }),
        ("sys", "login") => Some(SfsMessage::Login {
            zone: login_zone,
            nick,
            password,
        }),
        ("xt", "xtReq") => {
            // Parse the CDATA content as a nested XML dataObj
            let (cmd, params) = parse_extension_data(&body_text)?;
            Some(SfsMessage::ExtensionRequest { cmd, params })
        }
        _ => {
            tracing::warn!(msg_type, body_action, "Unknown SFS message type");
            None
        }
    }
}

/// Parse the `<dataObj>` content from an extension request.
/// Returns (cmd, params HashMap).
fn parse_extension_data(xml: &str) -> Option<(String, HashMap<String, SfsValue>)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut cmd = String::new();
    let mut params = HashMap::new();

    // We need to find:
    // <var n='cmd' t='s'>value</var>  — the command
    // <obj o='...' t='a'>...</obj>    — the params object (contains nested <var> elements)
    let mut in_var = false;
    let mut var_name = String::new();
    let mut var_type = String::new();
    let mut in_obj = false;
    let mut obj_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                match name {
                    "var" => {
                        in_var = true;
                        var_name.clear();
                        var_type.clear();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"n" => {
                                    var_name = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"t" => {
                                    var_type = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                _ => {}
                            }
                        }
                    }
                    "obj" => {
                        in_obj = true;
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"o" {
                                obj_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().unwrap_or_default().to_string();
                if in_var && !in_obj {
                    if var_name == "cmd" {
                        cmd = text;
                    }
                } else if in_var && in_obj {
                    let value = parse_sfs_value(&text, &var_type);
                    params.insert(var_name.clone(), value);
                }
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if in_var && !in_obj {
                    if var_name == "cmd" {
                        cmd = text;
                    }
                } else if in_var && in_obj {
                    let value = parse_sfs_value(&text, &var_type);
                    params.insert(var_name.clone(), value);
                }
            }
            Ok(Event::End(e)) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                match name {
                    "var" => in_var = false,
                    "obj" => {
                        in_obj = false;
                        let _ = &obj_name;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }

    Some((cmd, params))
}

/// Parse a string value into an SfsValue based on the type indicator.
fn parse_sfs_value(text: &str, type_str: &str) -> SfsValue {
    match type_str {
        "n" => SfsValue::Number(text.parse().unwrap_or(0.0)),
        "b" => SfsValue::Bool(text == "true" || text == "1"),
        _ => SfsValue::String(text.to_string()),
    }
}

// ── Encoding ─────────────────────────────────────────────────────────────────

/// Encode an extension message (server → client XT response).
///
/// Wraps a set of variables in the SFS extension message format:
/// `<msg t='xt'><body action='xtRes' r='-1'><![CDATA[<dataObj>...</dataObj>]]></body></msg>`
pub fn encode_extension(vars: &HashMap<String, SfsValue>) -> String {
    let mut inner = String::new();
    for (name, value) in vars {
        pack_variable(&mut inner, name, value);
    }
    format!(
        "<msg t='xt'><body action='xtRes' r='-1'><![CDATA[<dataObj>{inner}</dataObj>]]></body></msg>"
    )
}

/// Encode an extension message from raw XML variable content.
pub fn encode_extension_raw(vars_xml: &str) -> String {
    format!(
        "<msg t='xt'><body action='xtRes' r='-1'><![CDATA[<dataObj>{vars_xml}</dataObj>]]></body></msg>"
    )
}

/// Encode a system message.
pub fn encode_sys_message(action: &str, room: i32, body: &str) -> String {
    format!("<msg t='sys'><body action='{action}' r='{room}'>{body}</body></msg>")
}

/// Pack a single variable into XML.
fn pack_variable(out: &mut String, name: &str, value: &SfsValue) {
    match value {
        SfsValue::String(s) => {
            out.push_str(&format!("<var n='{name}' t='s'>{s}</var>"));
        }
        SfsValue::Number(n) => {
            // Format without trailing zeros for integers
            if *n == (*n as i64) as f64 {
                out.push_str(&format!("<var n='{name}' t='n'>{}</var>", *n as i64));
            } else {
                out.push_str(&format!("<var n='{name}' t='n'>{n}</var>"));
            }
        }
        SfsValue::Bool(b) => {
            out.push_str(&format!(
                "<var n='{name}' t='b'>{}</var>",
                if *b { "true" } else { "false" }
            ));
        }
        SfsValue::Object(map) => {
            out.push_str(&format!("<obj o='{name}' t='a'>"));
            for (k, v) in map {
                pack_variable(out, k, v);
            }
            out.push_str("</obj>");
        }
    }
}

/// Build a HashMap of SfsValues from key-value pairs.
/// Convenience macro for constructing extension messages.
#[macro_export]
macro_rules! sfs_vars {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = std::collections::HashMap::new();
        $(map.insert($key.to_string(), $val);)*
        map
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_check() {
        let xml = "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>";
        let msg = parse_message(xml).unwrap();
        match msg {
            SfsMessage::VersionCheck { version } => assert_eq!(version, 154),
            _ => panic!("Expected VersionCheck"),
        }
    }

    #[test]
    fn parse_login() {
        let xml = "<msg t='sys'><body action='login' r='0'><login z='Livewire'><nick><![CDATA[42]]></nick><pword><![CDATA[ABCD1234]]></pword></login></body></msg>";
        let msg = parse_message(xml).unwrap();
        match msg {
            SfsMessage::Login {
                zone,
                nick,
                password,
            } => {
                assert_eq!(zone, "Livewire");
                assert_eq!(nick, "42");
                assert_eq!(password, "ABCD1234");
            }
            _ => panic!("Expected Login"),
        }
    }

    #[test]
    fn parse_extension_request() {
        let xml = "<msg t='xt'><body action='xtReq' r='-1'><![CDATA[<dataObj><var n='cmd' t='s'>processmove</var><obj o='param' t='a'><var n='wirename' t='s'>g1</var></obj></dataObj>]]></body></msg>";
        let msg = parse_message(xml).unwrap();
        match msg {
            SfsMessage::ExtensionRequest { cmd, params } => {
                assert_eq!(cmd, "processmove");
                assert_eq!(params.get("wirename").unwrap().as_str(), Some("g1"));
            }
            _ => panic!("Expected ExtensionRequest"),
        }
    }

    #[test]
    fn encode_extension_roundtrip() {
        let mut vars = HashMap::new();
        vars.insert("_cmd".to_string(), SfsValue::String("victory".to_string()));
        vars.insert("score".to_string(), SfsValue::Number(100.0));
        let encoded = encode_extension(&vars);
        assert!(encoded.contains("<msg t='xt'>"));
        assert!(encoded.contains("victory"));
        assert!(encoded.contains("100"));
    }

    #[test]
    fn pack_nested_object() {
        let mut inner = HashMap::new();
        inner.insert("x".to_string(), SfsValue::Number(1.5));
        inner.insert("y".to_string(), SfsValue::Number(2.0));

        let mut vars = HashMap::new();
        vars.insert("pos".to_string(), SfsValue::Object(inner));

        let encoded = encode_extension(&vars);
        assert!(encoded.contains("<obj o='pos' t='a'>"));
        assert!(encoded.contains("<var n='x' t='n'>1.5</var>"));
    }
}
