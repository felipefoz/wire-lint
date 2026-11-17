//! Minimal bounded RSRC format-3 reader for the LVSR source-only flag.
//!
//! Offsets and bit semantics were researched using pylabview; see
//! THIRD_PARTY_NOTICES.md. No compiled-code blocks are interpreted or executed.

use super::invalid;
use std::io;

/// Extracts bit 10 of LVSR's big-endian `viFlags2` at payload offset 8.
///
/// Only format 3, LBVW-created VI/control resources with one LVSR section are
/// supported. Missing, ambiguous, or out-of-bounds records return an error.
pub(super) fn separate_compiled_code(bytes: &[u8]) -> io::Result<bool> {
    let first_header = slice(bytes, 0, 32)?;
    let mut header_offset = 0;
    let header = loop {
        let header = slice(bytes, header_offset, 32)?;
        if &header[..6] != b"RSRC\r\n"
            || &header[6..8] != b"\0\x03"
            || &header[12..16] != b"LBVW"
            || !matches!(&header[8..12], b"LVIN" | b"LVCC" | b"sVIN" | b"sVCC")
        {
            return Err(invalid("unsupported RSRC signature, version, or file type"));
        }
        if header[..16] != first_header[..16] || header[20..] != first_header[20..] {
            return Err(invalid("inconsistent RSRC headers"));
        }
        let next = number(header, 16)?;
        if next == header_offset {
            break header;
        }
        if next < header_offset + 32 {
            return Err(invalid("invalid RSRC header chain"));
        }
        header_offset = next;
    };

    let info = slice(bytes, header_offset, number(header, 20)?)?;
    let data_offset = number(header, 24)?;
    if data_offset < 32 {
        return Err(invalid("RSRC data overlaps the file header"));
    }
    let data = slice(bytes, data_offset, number(header, 28)?)?;
    if data_offset + data.len() > header_offset {
        return Err(invalid("RSRC data overlaps resource metadata"));
    }
    let list_header = slice(info, 32, 20)?;
    if number(list_header, 8)? != 32 {
        return Err(invalid("unsupported RSRC block-info header size"));
    }
    let table_offset = number(list_header, 12)?;
    if table_offset < 52 {
        return Err(invalid("invalid RSRC block table offset"));
    }
    let table = slice(info, table_offset, info.len().saturating_sub(table_offset))?;
    let count = number(table, 0)?
        .checked_add(1)
        .ok_or_else(|| invalid("RSRC block count overflow"))?;
    let table_size = count
        .checked_mul(12)
        .and_then(|size| size.checked_add(4))
        .ok_or_else(|| invalid("RSRC block table size overflow"))?;
    slice(table, 0, table_size)?;

    let mut flag = None;
    for block in table[4..table_size].as_chunks::<12>().0 {
        if &block[..4] != b"LVSR" {
            continue;
        }
        if flag.is_some() || number(block, 4)? != 0 {
            return Err(invalid("expected exactly one LVSR block and section"));
        }
        let section_offset = number(block, 8)?;
        if section_offset < table_size {
            return Err(invalid("LVSR section overlaps the block table"));
        }
        let section = slice(table, section_offset, 20)?;
        if number(section, 0)? != 0 {
            return Err(invalid("unsupported LVSR section index"));
        }
        let payload_offset = number(section, 12)?;
        let record = slice(
            data,
            payload_offset,
            data.len().saturating_sub(payload_offset),
        )?;
        let payload = slice(record, 4, number(record, 0)?)?;
        if payload.len() < 68 {
            return Err(invalid("truncated LVSR save record"));
        }
        flag = Some(number(payload, 8)? & (1 << 10) != 0);
    }
    flag.ok_or_else(|| invalid("missing LVSR save record"))
}

/// Returns a bounded byte range, rejecting overflow and truncated resources.
fn slice(bytes: &[u8], offset: usize, length: usize) -> io::Result<&[u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid("RSRC range overflow"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| invalid("RSRC range is outside its containing region"))
}

/// Reads one big-endian unsigned word as a checked host offset.
fn number(bytes: &[u8], offset: usize) -> io::Result<usize> {
    let word = slice(bytes, offset, 4)?;
    usize::try_from(u32::from_be_bytes([word[0], word[1], word[2], word[3]]))
        .map_err(|_| invalid("RSRC offset is too large for this platform"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a synthetic two-block RSRC with an unrelated block before LVSR.
    fn fixture(flags: u32) -> Vec<u8> {
        let data_size = 72;
        let info_offset = 32 + data_size;
        let info_size = 52 + 4 + 24 + 40;
        let mut bytes = vec![0; info_offset + info_size];
        for base in [0, info_offset] {
            bytes[base..base + 16].copy_from_slice(b"RSRC\r\n\0\x03LVINLBVW");
            put(&mut bytes, base + 16, info_offset as u32);
            put(&mut bytes, base + 20, info_size as u32);
            put(&mut bytes, base + 24, 32);
            put(&mut bytes, base + 28, data_size as u32);
        }
        put(&mut bytes, 32, 68);
        put(&mut bytes, 36, 0x15008000);
        put(&mut bytes, 44, flags);
        put(&mut bytes, info_offset + 40, 32);
        put(&mut bytes, info_offset + 44, 52);
        put(&mut bytes, info_offset + 48, info_size as u32);
        let table = info_offset + 52;
        put(&mut bytes, table, 1);
        bytes[table + 4..table + 8].copy_from_slice(b"TEST");
        put(&mut bytes, table + 12, 28);
        bytes[table + 16..table + 20].copy_from_slice(b"LVSR");
        put(&mut bytes, table + 24, 48);
        put(&mut bytes, table + 48 + 4, u32::MAX);
        bytes
    }

    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn reads_the_flag_not_the_presence_of_compiled_blocks() {
        for flags in [0, 1 << 9, 1 << 10, u32::MAX] {
            assert_eq!(
                separate_compiled_code(&fixture(flags)).unwrap(),
                flags & (1 << 10) != 0
            );
        }
        for extension in ["vi", "VIT", "vim", "ctl"] {
            assert!(
                super::super::read_facts(
                    std::path::Path::new(&format!("file.{extension}")),
                    &fixture(1 << 10),
                )
                .unwrap()
                .separate_compiled_code
            );
        }
    }

    #[test]
    fn reads_vi_control_and_template_resource_types() {
        for kind in [b"LVIN", b"LVCC", b"sVIN", b"sVCC"] {
            let mut bytes = fixture(1 << 10);
            for offset in [8, 104 + 8] {
                bytes[offset..offset + 4].copy_from_slice(kind);
            }
            assert!(separate_compiled_code(&bytes).unwrap());
        }
    }

    #[test]
    fn rejects_every_truncated_prefix_without_panicking() {
        let bytes = fixture(1 << 10);
        for length in 0..bytes.len() {
            assert!(separate_compiled_code(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn rejects_missing_ambiguous_and_invalid_resource_records() {
        let original = fixture(1 << 10);
        let table = 104 + 52;
        for (offset, value) in [
            (16, u32::MAX),
            (20, 1),
            (24, 0),
            (28, u32::MAX),
            (104 + 16, 0),
            (104 + 44, u32::MAX),
            (table, u32::MAX),
            (table + 16, u32::from_be_bytes(*b"NONE")),
            (table + 4, u32::from_be_bytes(*b"LVSR")),
            (table + 20, 1),
            (table + 24, 0),
            (table + 48, 1),
            (table + 60, u32::MAX),
            (32, 11),
            (32, u32::MAX),
        ] {
            let mut bytes = original.clone();
            put(&mut bytes, offset, value);
            assert!(separate_compiled_code(&bytes).is_err(), "offset {offset}");
        }
        let mut unsupported = original;
        unsupported[7] = 2;
        assert!(separate_compiled_code(&unsupported).is_err());
    }
}
