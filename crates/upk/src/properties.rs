//! UE3 tagged property parser.
//!
//! Reads self-describing properties from object serial data.
//! Format: FName name + FName type + i32 size + i32 array_index + [conditional extras] + value bytes.
//! Terminated by the "None" FName.

use crate::names::NameEntry;
use byteorder::{ByteOrder, LittleEndian};

/// A parsed tagged property value.
#[derive(Debug, Clone)]
pub enum PropValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(String),
    Name(String),
    Object(i32),
    Vector { x: f32, y: f32, z: f32 },
    Rotator { pitch: i32, yaw: i32, roll: i32 },
    Color { r: u8, g: u8, b: u8, a: u8 },
    LinearColor { r: f32, g: f32, b: f32, a: f32 },
    Array(Vec<u8>),
    Struct { struct_type: String, data: Vec<u8> },
    Byte(Vec<u8>),
    Raw { type_name: String, data: Vec<u8> },
}

/// A single tagged property with its name and value.
#[derive(Debug, Clone)]
pub struct TaggedProperty {
    pub name: String,
    pub array_index: i32,
    pub value: PropValue,
}

/// Parse tagged properties from a byte slice starting at `offset`.
/// `names` is the package's name table for resolving FName references.
///
/// Returns a list of properties. Stops at the "None" terminator or end of data.
pub fn parse_tagged_properties(
    data: &[u8],
    offset: usize,
    names: &[NameEntry],
) -> Vec<TaggedProperty> {
    parse_tagged_properties_with_end(data, offset, names).0
}

/// Like [`parse_tagged_properties`] but also returns the byte offset immediately
/// after the "None" terminator (i.e., where class-specific binary data begins).
pub fn parse_tagged_properties_with_end(
    data: &[u8],
    offset: usize,
    names: &[NameEntry],
) -> (Vec<TaggedProperty>, usize) {
    let mut props = Vec::new();
    let mut pos = offset;

    while pos + 8 <= data.len() {
        // Read property name FName
        let name_idx = LittleEndian::read_i32(&data[pos..]) as usize;
        let _name_num = LittleEndian::read_i32(&data[pos + 4..]);
        pos += 8;

        if name_idx >= names.len() {
            break;
        }

        let name = &names[name_idx].name;
        if name == "None" {
            break;
        }

        if pos + 8 > data.len() {
            break;
        }

        // Read type FName
        let type_idx = LittleEndian::read_i32(&data[pos..]) as usize;
        pos += 8;
        let type_name = if type_idx < names.len() {
            &names[type_idx].name
        } else {
            break;
        };

        if pos + 8 > data.len() {
            break;
        }

        // Read size and array index
        let prop_size = LittleEndian::read_i32(&data[pos..]) as usize;
        pos += 4;
        let array_idx = LittleEndian::read_i32(&data[pos..]);
        pos += 4;

        // Handle type-specific extra tags
        let mut struct_type = String::new();

        if type_name == "StructProperty" {
            if pos + 8 > data.len() {
                break;
            }
            let si = LittleEndian::read_i32(&data[pos..]) as usize;
            pos += 8;
            if si < names.len() {
                struct_type = names[si].name.clone();
            }
        }

        if type_name == "BoolProperty" {
            if pos + 4 > data.len() {
                break;
            }
            let bool_val = LittleEndian::read_i32(&data[pos..]);
            pos += 4;
            props.push(TaggedProperty {
                name: name.clone(),
                array_index: array_idx,
                value: PropValue::Bool(bool_val != 0),
            });
            continue;
        }

        if type_name == "ByteProperty" {
            // Check if there's an extra enum FName by validating the name index.
            // UE3 VER_ADDED_ENUM_NAME_TO_BYTE_PROPERTY_TAG varies by licensee.
            // SGW v486: enum FName is present but only if the next 4 bytes decode
            // to a valid name index. If invalid, skip nothing.
            if pos + 8 <= data.len() {
                let enum_name_idx = LittleEndian::read_i32(&data[pos..]) as usize;
                if enum_name_idx < names.len() {
                    pos += 8;
                }
            }
        }

        // Read value data
        if pos + prop_size > data.len() {
            break;
        }
        let value_data = &data[pos..pos + prop_size];
        pos += prop_size;

        let value = match type_name.as_str() {
            "FloatProperty" if prop_size == 4 => {
                PropValue::Float(LittleEndian::read_f32(value_data))
            }
            "IntProperty" if prop_size == 4 => PropValue::Int(LittleEndian::read_i32(value_data)),
            "ObjectProperty" if prop_size == 4 => {
                PropValue::Object(LittleEndian::read_i32(value_data))
            }
            "NameProperty" if prop_size == 8 => {
                let ni = LittleEndian::read_i32(value_data) as usize;
                let n = if ni < names.len() {
                    names[ni].name.clone()
                } else {
                    format!("?{}", ni)
                };
                PropValue::Name(n)
            }
            "StrProperty" => PropValue::Str(parse_fstring_from_bytes(value_data)),
            "StructProperty" => match struct_type.as_str() {
                "Vector" if prop_size == 12 => PropValue::Vector {
                    x: LittleEndian::read_f32(&value_data[0..]),
                    y: LittleEndian::read_f32(&value_data[4..]),
                    z: LittleEndian::read_f32(&value_data[8..]),
                },
                "Rotator" if prop_size == 12 => PropValue::Rotator {
                    pitch: LittleEndian::read_i32(&value_data[0..]),
                    yaw: LittleEndian::read_i32(&value_data[4..]),
                    roll: LittleEndian::read_i32(&value_data[8..]),
                },
                "Color" if prop_size == 4 => PropValue::Color {
                    b: value_data[0],
                    g: value_data[1],
                    r: value_data[2],
                    a: value_data[3],
                },
                "LinearColor" if prop_size == 16 => PropValue::LinearColor {
                    r: LittleEndian::read_f32(&value_data[0..]),
                    g: LittleEndian::read_f32(&value_data[4..]),
                    b: LittleEndian::read_f32(&value_data[8..]),
                    a: LittleEndian::read_f32(&value_data[12..]),
                },
                _ => PropValue::Struct {
                    struct_type: struct_type.clone(),
                    data: value_data.to_vec(),
                },
            },
            "ArrayProperty" => PropValue::Array(value_data.to_vec()),
            "ByteProperty" => PropValue::Byte(value_data.to_vec()),
            _ => PropValue::Raw {
                type_name: type_name.clone(),
                data: value_data.to_vec(),
            },
        };

        props.push(TaggedProperty {
            name: name.clone(),
            array_index: array_idx,
            value,
        });
    }

    (props, pos)
}

fn parse_fstring_from_bytes(data: &[u8]) -> String {
    if data.len() < 4 {
        return String::new();
    }
    let length = LittleEndian::read_i32(data);
    if length == 0 {
        return String::new();
    }
    if length > 0 {
        let len = length as usize;
        if data.len() >= 4 + len {
            let s = &data[4..4 + len];
            let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
            String::from_utf8_lossy(&s[..end]).to_string()
        } else {
            String::new()
        }
    } else {
        let count = (-length) as usize;
        if data.len() >= 4 + count * 2 {
            let chars: Vec<u16> = data[4..4 + count * 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&chars)
                .trim_end_matches('\0')
                .to_string()
        } else {
            String::new()
        }
    }
}
