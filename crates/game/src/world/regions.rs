use serde::{Deserialize, Serialize};

use cimmeria_common::math::Vector3;

/// Shape of a region boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegionShape {
    /// A spherical region defined by center + radius.
    Sphere { center: Vector3, radius: f32 },
    /// An axis-aligned bounding box.
    Box { min: Vector3, max: Vector3 },
}

/// A named region in a game space.
///
/// Corresponds to the generic_regions table. Regions fire enter/exit
/// events when entities cross their boundaries, which missions and
/// scripts can listen for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub region_id: i32,
    pub name: String,
    pub space_id: i32,
    pub shape: RegionShape,
    pub flags: u32,
}

impl Region {
    /// Create a new spherical region.
    pub fn sphere(region_id: i32, name: String, space_id: i32, center: Vector3, radius: f32) -> Self {
        Self {
            region_id,
            name,
            space_id,
            shape: RegionShape::Sphere { center, radius },
            flags: 0,
        }
    }

    /// Create a new box region.
    pub fn aabb(region_id: i32, name: String, space_id: i32, min: Vector3, max: Vector3) -> Self {
        Self {
            region_id,
            name,
            space_id,
            shape: RegionShape::Box { min, max },
            flags: 0,
        }
    }

    /// Test whether a point is inside this region.
    pub fn contains(&self, point: &Vector3) -> bool {
        match &self.shape {
            RegionShape::Sphere { center, radius } => {
                center.distance_squared_to(point) <= radius * radius
            }
            RegionShape::Box { min, max } => {
                point.x >= min.x
                    && point.x <= max.x
                    && point.y >= min.y
                    && point.y <= max.y
                    && point.z >= min.z
                    && point.z <= max.z
            }
        }
    }

    /// Detect a boundary crossing between two positions.
    /// Returns `Some(true)` for enter, `Some(false)` for exit, `None` for no crossing.
    pub fn check_crossing(&self, old_pos: &Vector3, new_pos: &Vector3) -> Option<bool> {
        let was_inside = self.contains(old_pos);
        let is_inside = self.contains(new_pos);

        if !was_inside && is_inside {
            Some(true) // entered
        } else if was_inside && !is_inside {
            Some(false) // exited
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_containment() {
        let r = Region::sphere(1, "gate_room".to_string(), 1, Vector3::zero(), 10.0);
        assert!(r.contains(&Vector3::new(5.0, 0.0, 0.0)));
        assert!(!r.contains(&Vector3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn box_containment() {
        let r = Region::aabb(
            2,
            "corridor".to_string(),
            1,
            Vector3::new(-5.0, -5.0, -5.0),
            Vector3::new(5.0, 5.0, 5.0),
        );
        assert!(r.contains(&Vector3::zero()));
        assert!(!r.contains(&Vector3::new(10.0, 0.0, 0.0)));
    }

    #[test]
    fn crossing_detection() {
        let r = Region::sphere(1, "gate_room".to_string(), 1, Vector3::zero(), 10.0);
        let outside = Vector3::new(20.0, 0.0, 0.0);
        let inside = Vector3::new(5.0, 0.0, 0.0);

        assert_eq!(r.check_crossing(&outside, &inside), Some(true));
        assert_eq!(r.check_crossing(&inside, &outside), Some(false));
        assert_eq!(r.check_crossing(&inside, &inside), None);
    }
}
