//! Shared types extracted from .def XML files.

/// A single property extracted from a .def file's <Properties> section.
#[derive(Debug, Clone)]
pub struct DefProperty {
    /// Property name (the XML tag name, e.g. "playerName").
    pub name: String,
    /// BigWorld type string (e.g. "INT32", "WSTRING", "VECTOR3").
    pub bw_type: String,
    /// Flags value (e.g. "BASE", "CELL_PUBLIC", "CELL_PRIVATE").
    pub flags: String,
}

/// Method signature extracted from a .def file.
#[derive(Debug, Clone)]
pub struct DefMethod {
    /// Method name (the XML tag name).
    pub name: String,
    /// Whether this method is exposed to the client.
    pub exposed: bool,
    /// Argument types (BigWorld type strings).
    pub arg_types: Vec<String>,
}

/// All data parsed from a single .def file.
#[derive(Debug)]
pub struct EntityDef {
    /// Entity type name (e.g. "SGWPlayer").
    pub name: String,
    /// Parent entity type, if any.
    pub parent: Option<String>,
    /// Interface names this entity implements.
    pub interfaces: Vec<String>,
    /// Properties declared in <Properties>.
    pub properties: Vec<DefProperty>,
    /// Client-callable methods.
    pub client_methods: Vec<DefMethod>,
    /// Cell-side methods.
    pub cell_methods: Vec<DefMethod>,
    /// Base-side methods.
    pub base_methods: Vec<DefMethod>,
}
