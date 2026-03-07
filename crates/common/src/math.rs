use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

/// 3D position/direction vector using single-precision floats.
///
/// Used throughout the engine for entity positions, movement vectors,
/// and spatial calculations. Corresponds to the `Vector3` type used
/// in BigWorld position updates and the CellApp spatial grid.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    /// Create a new vector with the given components.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// The zero vector (0, 0, 0).
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Euclidean distance to another point.
    pub fn distance_to(&self, other: &Vector3) -> f32 {
        self.distance_squared_to(other).sqrt()
    }

    /// Squared Euclidean distance to another point.
    ///
    /// Prefer this over `distance_to` when comparing distances, since it
    /// avoids the square root computation.
    pub fn distance_squared_to(&self, other: &Vector3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Length (magnitude) of the vector.
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Returns a unit vector in the same direction, or the zero vector
    /// if the length is effectively zero.
    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len < f32::EPSILON {
            Self::zero()
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        }
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

/// Rotation quaternion using single-precision floats.
///
/// Used for entity orientation in the game world. The BigWorld engine
/// transmits orientation as yaw/pitch/roll in position updates, but
/// internally quaternions are used for interpolation and rotation math.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    /// The identity quaternion (no rotation).
    pub fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    /// Create a new quaternion with the given components.
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector3_zero() {
        let v = Vector3::zero();
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
        assert_eq!(v.z, 0.0);
    }

    #[test]
    fn vector3_new() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn vector3_add() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(4.0, 5.0, 6.0);
        let c = a + b;
        assert_eq!(c, Vector3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn vector3_sub() {
        let a = Vector3::new(4.0, 5.0, 6.0);
        let b = Vector3::new(1.0, 2.0, 3.0);
        let c = a - b;
        assert_eq!(c, Vector3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn vector3_mul_scalar() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        let scaled = v * 2.0;
        assert_eq!(scaled, Vector3::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn vector3_length() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        assert!((v.length() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vector3_distance() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(3.0, 4.0, 0.0);
        assert!((a.distance_to(&b) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vector3_distance_squared() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(3.0, 4.0, 0.0);
        assert!((a.distance_squared_to(&b) - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vector3_normalized() {
        let v = Vector3::new(3.0, 0.0, 0.0);
        let n = v.normalized();
        assert!((n.x - 1.0).abs() < f32::EPSILON);
        assert!((n.y - 0.0).abs() < f32::EPSILON);
        assert!((n.z - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vector3_normalized_zero() {
        let v = Vector3::zero();
        let n = v.normalized();
        assert_eq!(n, Vector3::zero());
    }

    #[test]
    fn quaternion_identity() {
        let q = Quaternion::identity();
        assert_eq!(q.x, 0.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
        assert_eq!(q.w, 1.0);
    }

    #[test]
    fn quaternion_new() {
        let q = Quaternion::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(q.x, 0.1);
        assert_eq!(q.y, 0.2);
        assert_eq!(q.z, 0.3);
        assert_eq!(q.w, 0.4);
    }

    #[test]
    fn default_vector3_is_zero() {
        let v = Vector3::default();
        assert_eq!(v, Vector3::zero());
    }

    #[test]
    fn default_quaternion_is_zero_components() {
        let q = Quaternion::default();
        assert_eq!(q.x, 0.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
        assert_eq!(q.w, 0.0);
    }
}
