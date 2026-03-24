//! Raw FFI bindings to the Detour C wrapper (detour_wrapper.h).
//!
//! These are unsafe C functions — all safety invariants are enforced
//! by the `NavMesh` struct in `navigation.rs`.

use std::ffi::c_void;

extern "C" {
    pub fn detour_build_navmesh_data(
        verts: *const u16,
        nverts: i32,
        polys: *const u16,
        npolys: i32,
        nvp: i32,
        flags: *const u16,
        areas: *const u8,
        bmin: *const f32,
        bmax: *const f32,
        cs: f32,
        ch: f32,
        agent_height: f32,
        agent_radius: f32,
        agent_max_climb: f32,
        detail_meshes: *const u32,
        ndetail_meshes: i32,
        detail_verts: *const f32,
        ndetail_verts: i32,
        detail_tris: *const u8,
        ndetail_tris: i32,
        out_data: *mut *mut u8,
        out_data_size: *mut i32,
    ) -> i32;

    pub fn detour_free_data(data: *mut u8);

    pub fn detour_create_navmesh(data: *const u8, data_size: i32) -> *mut c_void;
    pub fn detour_free_navmesh(handle: *mut c_void);

    pub fn detour_create_query(mesh: *mut c_void, max_nodes: i32) -> *mut c_void;
    pub fn detour_free_query(handle: *mut c_void);

    pub fn detour_find_nearest_poly(
        query: *mut c_void,
        center: *const f32,
        extents: *const f32,
        nearest_ref: *mut u32,
        nearest_pt: *mut f32,
    ) -> u32;

    pub fn detour_find_path(
        query: *mut c_void,
        start_ref: u32,
        end_ref: u32,
        start_pos: *const f32,
        end_pos: *const f32,
        path: *mut u32,
        path_count: *mut i32,
        max_path: i32,
    ) -> u32;

    pub fn detour_find_straight_path(
        query: *mut c_void,
        start_pos: *const f32,
        end_pos: *const f32,
        path: *const u32,
        path_count: i32,
        straight_path: *mut f32,
        straight_path_count: *mut i32,
        max_straight_path: i32,
    ) -> u32;

    pub fn detour_raycast(
        query: *mut c_void,
        start_ref: u32,
        start_pos: *const f32,
        end_pos: *const f32,
        hit_normal: *mut f32,
        t: *mut f32,
    ) -> i32;

    pub fn detour_get_poly_height(
        query: *mut c_void,
        poly_ref: u32,
        pos: *const f32,
        height: *mut f32,
    ) -> u32;

    pub fn detour_closest_point_on_poly(
        query: *mut c_void,
        poly_ref: u32,
        pos: *const f32,
        closest: *mut f32,
    ) -> u32;

    pub fn detour_move_along_surface(
        query: *mut c_void,
        start_ref: u32,
        start_pos: *const f32,
        end_pos: *const f32,
        result_pos: *mut f32,
        visited: *mut u32,
        visited_count: *mut i32,
        max_visited: i32,
    ) -> u32;
}

/// Detour status flag: operation failed.
pub const DT_FAILURE: u32 = 1 << 31;

/// Check if a dtStatus indicates failure.
#[inline]
pub fn dt_status_failed(status: u32) -> bool {
    status & DT_FAILURE != 0
}
