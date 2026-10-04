//! Signed inert ZIP inputs shared by engine and host integration tests.
use super::*;
use crate::catalog::verify_release;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::io::Cursor;
pub fn archive(with_exe: bool) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    zip.start_file(
        if with_exe {
            "Working/Binaries/SGW.exe"
        } else {
            "unrelated.txt"
        },
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(&exe).unwrap();
    zip.start_file("later.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"later entry").unwrap();
    zip.finish().unwrap().into_inner()
}
pub fn verified(seed: &[u8]) -> VerifiedRelease {
    let hash: String = Sha256::digest(seed)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":seed.len(),"sha256":hash},"patches":[]})).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let signature: String = signature
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    verify_release(&body, signature.as_bytes()).unwrap()
}
