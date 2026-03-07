// generator/parser.rs - XML .def file parser placeholder.
//
// The parsing logic for entities.xml and individual .def files currently
// lives inline in build.rs (see parse_entities_xml and parse_def_file).
//
// TODO: If this crate is refactored to use a separate codegen library crate
// (e.g. cimmeria-defs-codegen), move the parsing functions here. The parser
// should handle:
//
// - entities.xml: Extract the list of registered entity type names
// - *.def files: Extract <Properties>, <ClientMethods>, <CellMethods>,
//   <BaseMethods>, <Parent>, <Implements>, and <Volatile> sections
// - BigWorld type mapping (INT32, WSTRING, VECTOR3, ARRAY<of>T</of>, etc.)
// - Property flags (BASE, CELL_PUBLIC, CELL_PRIVATE, ALL_CLIENTS, etc.)
// - Method arguments with <Arg> type extraction and <Exposed/> detection
