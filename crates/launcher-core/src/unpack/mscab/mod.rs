//! A native reader for the 2009 installer's MakeCAB cabinet set, for hosts
//! without FDI. It parses the cabinet headers and decodes MSZIP blocks.
//! Spanning sets, folder planning and extraction come in later modules.

#![allow(dead_code, unused_imports)] // wired in LI-04

mod header;
mod mszip;

pub(super) use header::{
    read_cabinet, read_data_block, Cabinet, DataBlock, FileEntry, Folder, CONTINUED_FROM_PREV,
    CONTINUED_TO_NEXT, PREV_AND_NEXT, TYPE_MSZIP, TYPE_NONE,
};
pub(super) use mszip::{MsZip, BLOCK_MAX};
