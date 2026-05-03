//! XML loading: parse `spaces.xml` (world definitions) and `cell_spaces.xml`
//! (startup space list).

use super::{SpaceManager, WorldDef};

impl SpaceManager {
    /// Load world definitions from `spaces.xml` and create startup spaces from
    /// `cell_spaces.xml`. Both files are expected in `entities_dir`.
    pub fn load_from_xml(&mut self, entities_dir: &str) -> Result<(), String> {
        // Load world definitions from spaces.xml
        let spaces_path = format!("{}/spaces.xml", entities_dir);
        let spaces_xml = std::fs::read_to_string(&spaces_path)
            .map_err(|e| format!("Failed to read {spaces_path}: {e}"))?;
        self.parse_spaces_xml(&spaces_xml)?;

        // Load startup space list from cell_spaces.xml
        let cell_spaces_path = format!("{}/cell_spaces.xml", entities_dir);
        let cell_spaces_xml = std::fs::read_to_string(&cell_spaces_path)
            .map_err(|e| format!("Failed to read {cell_spaces_path}: {e}"))?;
        self.create_startup_spaces(&cell_spaces_xml)?;

        Ok(())
    }

    /// Parse `spaces.xml` and populate the `worlds` map.
    pub(crate) fn parse_spaces_xml(&mut self, xml: &str) -> Result<(), String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                    if e.name().as_ref() == b"Space" =>
                {
                    let mut world_name = String::new();
                    let mut instanced = false;
                    let mut min_x: i32 = 0;
                    let mut max_x: i32 = 0;
                    let mut min_y: i32 = 0;
                    let mut max_y: i32 = 0;

                    for attr_res in e.attributes() {
                        let attr = attr_res.map_err(|err| {
                            format!("spaces.xml: malformed Space attribute: {err}")
                        })?;
                        let key = std::str::from_utf8(attr.key.as_ref())
                            .map_err(|err| format!("spaces.xml: non-UTF8 attribute key: {err}"))?;
                        let val = std::str::from_utf8(&attr.value).map_err(|err| {
                            format!("spaces.xml: non-UTF8 value for {key}: {err}")
                        })?;
                        match key {
                            "WorldName" => world_name = val.to_string(),
                            "Instanced" => instanced = val == "true",
                            "MinX" => {
                                min_x = val.parse().map_err(|err| {
                                    format!("spaces.xml: MinX={val:?} not a valid i32: {err}")
                                })?
                            }
                            "MaxX" => {
                                max_x = val.parse().map_err(|err| {
                                    format!("spaces.xml: MaxX={val:?} not a valid i32: {err}")
                                })?
                            }
                            "MinY" => {
                                min_y = val.parse().map_err(|err| {
                                    format!("spaces.xml: MinY={val:?} not a valid i32: {err}")
                                })?
                            }
                            "MaxY" => {
                                max_y = val.parse().map_err(|err| {
                                    format!("spaces.xml: MaxY={val:?} not a valid i32: {err}")
                                })?
                            }
                            _ => {}
                        }
                    }

                    if !world_name.is_empty() {
                        tracing::trace!(
                            world = %world_name,
                            instanced,
                            "Loaded world definition"
                        );
                        self.worlds.insert(
                            world_name.clone(),
                            WorldDef {
                                world_name,
                                instanced,
                                min_x,
                                max_x,
                                min_y,
                                max_y,
                            },
                        );
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("XML parse error: {e}")),
                _ => {}
            }
            buf.clear();
        }

        tracing::info!(
            count = self.worlds.len(),
            "Parsed world definitions from spaces.xml"
        );
        Ok(())
    }

    /// Parse `cell_spaces.xml` and create a space instance for each listed world.
    pub(crate) fn create_startup_spaces(&mut self, xml: &str) -> Result<(), String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                    if e.name().as_ref() == b"Space" =>
                {
                    let mut world_name = String::new();
                    for attr_res in e.attributes() {
                        let attr = attr_res.map_err(|err| {
                            format!("cell_spaces.xml: malformed Space attribute: {err}")
                        })?;
                        let key = std::str::from_utf8(attr.key.as_ref()).map_err(|err| {
                            format!("cell_spaces.xml: non-UTF8 attribute key: {err}")
                        })?;
                        let val = std::str::from_utf8(&attr.value).map_err(|err| {
                            format!("cell_spaces.xml: non-UTF8 value for {key}: {err}")
                        })?;
                        if key == "WorldName" {
                            world_name = val.to_string();
                        }
                    }

                    if !world_name.is_empty() {
                        if !self.worlds.contains_key(&world_name) {
                            tracing::warn!(world = %world_name, "Startup space references unknown world — skipping");
                            continue;
                        }
                        let space_id = self.allocate_space_id();
                        self.create_space_instance(space_id, &world_name);
                        self.world_spaces.insert(world_name, space_id);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("XML parse error: {e}")),
                _ => {}
            }
            buf.clear();
        }

        tracing::info!(
            count = self.spaces.len(),
            id_range = format_args!(
                "{}..{}",
                (self.cell_id as u32) << 16,
                ((self.cell_id as u32) << 16) | (self.next_local_id.saturating_sub(1))
            ),
            "Created startup spaces from cell_spaces.xml"
        );
        Ok(())
    }
}
