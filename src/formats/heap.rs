//! Bounded decoding of version-b block-diagram heaps for free-label bookmarks
//! and front-panel terminal display styles.

use super::{invalid, rsrc};
use flate2::bufread::ZlibDecoder;
use std::io::{self, Read};

const MAX_HEAP_SIZE: usize = 64 * 1024 * 1024;
const ARRAY_ELEMENT: i32 = -6;
const CLASS: i32 = -2;
const DIAGRAM: i32 = 27;
const LABEL: i32 = 10;
const TEXT_HAIR: i32 = 1;
const NODE_LIST: i32 = 164;
const Z_PLANE_LIST: i32 = 303;
const TEXT_RECORD: i32 = 270;
const TEXT: i32 = 3;
/// `SL__fPTerm`: a front-panel control/indicator's terminal on the block diagram.
const FP_TERM: i32 = 22;
/// `OF__objFlags`: the terminal's object-flags field, including its display style.
const OBJ_FLAGS: i32 = 172;
/// Terminal object-flags bit selecting icon-style display; clear is data-type style.
const ICON_STYLE_FLAG: u32 = 0x0400_0000;

#[derive(Clone, Copy)]
struct Node {
    tag: i32,
    class: Option<i32>,
}

/// Reads the sole compressed BDHb section, rejecting malformed or oversized heaps.
///
/// Returns free-label bookmarks and, in terminal order, each front-panel
/// terminal's icon-style flag (`true` for icon style, `false` for data-type
/// style).
pub(super) fn read_bdhb(
    table: &[u8],
    table_size: usize,
    data: &[u8],
    count_minus_one: usize,
    section_offset: usize,
) -> io::Result<(Vec<Vec<u8>>, Vec<bool>)> {
    if count_minus_one != 0 || section_offset < table_size {
        return Err(invalid("unsupported BDHb section table"));
    }
    let section = rsrc::slice(table, section_offset, 20)?;
    if rsrc::number(section, 0)? != 0 {
        return Err(invalid("unsupported BDHb section index"));
    }
    let offset = rsrc::number(section, 12)?;
    let record = rsrc::slice(data, offset, data.len().saturating_sub(offset))?;
    let payload = rsrc::slice(record, 4, rsrc::number(record, 0)?)?;
    let expected = rsrc::number(payload, 0)?;
    if expected > MAX_HEAP_SIZE {
        return Err(invalid("BDHb heap exceeds the size limit"));
    }
    let compressed = &payload[4..];
    let mut decoder = ZlibDecoder::new(compressed);
    let mut decoded = Vec::new();
    decoder
        .by_ref()
        .take((MAX_HEAP_SIZE + 1) as u64)
        .read_to_end(&mut decoded)
        .map_err(|error| invalid(format!("invalid compressed BDHb heap: {error}")))?;
    if decoded.len() != expected || decoder.total_in() as usize != compressed.len() {
        return Err(invalid("inconsistent compressed BDHb heap length"));
    }
    let length = rsrc::number(&decoded, 0)?;
    if length != decoded.len() - 4 {
        return Err(invalid("inconsistent BDHb heap length"));
    }
    parse_heap(&decoded[4..])
}

/// Walks heap scopes so only diagram-owned free labels and terminal objects
/// contribute facts.
fn parse_heap(bytes: &[u8]) -> io::Result<(Vec<Vec<u8>>, Vec<bool>)> {
    let mut cursor = 0;
    let mut stack: Vec<Node> = Vec::new();
    let mut bookmarks = Vec::new();
    let mut terminal_icon_styles = Vec::new();
    while cursor < bytes.len() {
        let header = take(bytes, &mut cursor, 2)?;
        let size = header[0] >> 5;
        let attributes = header[0] & 0x10 != 0;
        let scope = (header[0] >> 2) & 3;
        let raw_tag = (((header[0] & 3) as i32) << 8) | header[1] as i32;
        let tag = if raw_tag == 1023 {
            signed(bytes, &mut cursor, 4)?
        } else {
            raw_tag - 31
        };
        let mut class = None;
        if attributes {
            let count = unsigned_variable(bytes, &mut cursor)?;
            for _ in 0..count {
                let id = signed_variable(bytes, &mut cursor)?;
                let value = signed_short_variable(bytes, &mut cursor)?;
                if id == CLASS && class.replace(value).is_some() {
                    return Err(invalid("duplicate BDHb class attribute"));
                }
            }
        }
        let length = match size {
            0 | 7 => 0,
            1..=4 => size as usize,
            6 => unsigned_variable(bytes, &mut cursor)?,
            _ => return Err(invalid("unsupported BDHb heap value size")),
        };
        let content = take(bytes, &mut cursor, length)?;
        match scope {
            0 | 1 => {
                if tag == TEXT && stack.len() >= 4 {
                    let parents = &stack[stack.len() - 4..];
                    if parents[0].class == Some(DIAGRAM)
                        && matches!(parents[1].tag, NODE_LIST | Z_PLANE_LIST)
                        && parents[2].tag == ARRAY_ELEMENT
                        && parents[2].class == Some(LABEL)
                        && parents[3].tag == TEXT_RECORD
                        && parents[3].class == Some(TEXT_HAIR)
                        && content.starts_with(b"#")
                    {
                        bookmarks.push(content.to_vec());
                    }
                }
                if tag == OBJ_FLAGS
                    && stack.last().is_some_and(|parent| {
                        parent.tag == ARRAY_ELEMENT && parent.class == Some(FP_TERM)
                    })
                {
                    terminal_icon_styles.push(object_flags(content)? & ICON_STYLE_FLAG != 0);
                }
                if scope == 0 {
                    stack.push(Node { tag, class });
                }
            }
            2 => {
                let parent = stack
                    .pop()
                    .ok_or_else(|| invalid("unmatched BDHb closing tag"))?;
                if parent.tag != tag {
                    return Err(invalid("mismatched BDHb closing tag"));
                }
            }
            _ => return Err(invalid("unsupported BDHb heap scope")),
        }
    }
    if !stack.is_empty() {
        return Err(invalid("unclosed BDHb heap scope"));
    }
    Ok((bookmarks, terminal_icon_styles))
}

/// Reads a terminal's `objFlags` field as a big-endian unsigned integer.
fn object_flags(content: &[u8]) -> io::Result<u32> {
    if content.len() > 4 {
        return Err(invalid("oversized BDHb terminal objFlags field"));
    }
    let mut bytes = [0; 4];
    bytes[4 - content.len()..].copy_from_slice(content);
    Ok(u32::from_be_bytes(bytes))
}

/// Advances through a bounded heap range.
fn take<'a>(bytes: &'a [u8], cursor: &mut usize, length: usize) -> io::Result<&'a [u8]> {
    let value = rsrc::slice(bytes, *cursor, length)?;
    *cursor += length;
    Ok(value)
}

/// Reads a signed big-endian integer of a supported width.
fn signed(bytes: &[u8], cursor: &mut usize, length: usize) -> io::Result<i32> {
    let value = take(bytes, cursor, length)?;
    Ok(match length {
        1 => i8::from_be_bytes([value[0]]) as i32,
        2 => i16::from_be_bytes([value[0], value[1]]) as i32,
        4 => i32::from_be_bytes(value.try_into().unwrap()),
        _ => unreachable!(),
    })
}

/// Reads LabVIEW's unsigned 1/2/4-byte variable field.
fn unsigned_variable(bytes: &[u8], cursor: &mut usize) -> io::Result<usize> {
    let first = take(bytes, cursor, 1)?[0];
    let value = match first {
        255 => u16::from_be_bytes(take(bytes, cursor, 2)?.try_into().unwrap()) as u32,
        254 => u32::from_be_bytes(take(bytes, cursor, 4)?.try_into().unwrap()),
        _ => first as u32,
    };
    usize::try_from(value).map_err(|_| invalid("BDHb value is too large"))
}

/// Reads LabVIEW's signed 1/2/4-byte attribute identifier.
fn signed_variable(bytes: &[u8], cursor: &mut usize) -> io::Result<i32> {
    match signed(bytes, cursor, 1)? {
        -128 => signed(bytes, cursor, 2),
        -127 => signed(bytes, cursor, 4),
        value => Ok(value),
    }
}

/// Reads a signed 2/4-byte attribute value.
fn signed_short_variable(bytes: &[u8], cursor: &mut usize) -> io::Result<i32> {
    match signed(bytes, cursor, 2)? {
        -32768 => signed(bytes, cursor, 4),
        value => Ok(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(heap: &mut Vec<u8>, tag: i32, scope: u8, class: Option<i16>, content: &[u8]) {
        let tag = (tag + 31) as u16;
        let size = if content.is_empty() { 0 } else { 6 };
        heap.push((size << 5) | (u8::from(class.is_some()) << 4) | (scope << 2) | (tag >> 8) as u8);
        heap.push(tag as u8);
        if let Some(class) = class {
            heap.extend_from_slice(&[1, 0xfe]);
            heap.extend_from_slice(&class.to_be_bytes());
        }
        if !content.is_empty() {
            heap.push(content.len() as u8);
            heap.extend_from_slice(content);
        }
    }

    fn label(heap: &mut Vec<u8>, parent: i32, text: &[u8]) {
        node(heap, parent, 0, None, &[]);
        node(heap, ARRAY_ELEMENT, 0, Some(LABEL as i16), &[]);
        node(heap, TEXT_RECORD, 0, Some(TEXT_HAIR as i16), &[]);
        node(heap, TEXT, 1, None, text);
        node(heap, TEXT_RECORD, 2, None, &[]);
        node(heap, ARRAY_ELEMENT, 2, None, &[]);
        node(heap, parent, 2, None, &[]);
    }

    fn terminal(heap: &mut Vec<u8>, object_flags: u32) {
        node(heap, ARRAY_ELEMENT, 0, Some(FP_TERM as i16), &[]);
        node(heap, OBJ_FLAGS, 1, None, &object_flags.to_be_bytes());
        node(heap, ARRAY_ELEMENT, 2, None, &[]);
    }

    #[test]
    fn reads_only_diagram_free_labels_including_nested_diagrams() {
        let mut heap = Vec::new();
        node(&mut heap, -7, 0, Some(DIAGRAM as i16), &[]);
        label(&mut heap, Z_PLANE_LIST, b"#wirelint file-description");
        label(&mut heap, 200, b"#wirelint no-breakpoints");
        label(&mut heap, NODE_LIST, b"not #a-bookmark");
        node(&mut heap, 63, 0, None, &[]);
        node(&mut heap, ARRAY_ELEMENT, 0, Some(DIAGRAM as i16), &[]);
        label(&mut heap, Z_PLANE_LIST, b"#other");
        node(&mut heap, ARRAY_ELEMENT, 2, None, &[]);
        node(&mut heap, 63, 2, None, &[]);
        node(&mut heap, -7, 2, None, &[]);
        let (bookmarks, terminal_icon_styles) = parse_heap(&heap).unwrap();
        assert_eq!(
            bookmarks,
            vec![b"#wirelint file-description".to_vec(), b"#other".to_vec()]
        );
        assert!(terminal_icon_styles.is_empty());
        heap.pop();
        assert!(parse_heap(&heap).is_err());
    }

    #[test]
    fn reads_terminal_icon_styles_in_order_alongside_bookmarks() {
        let mut heap = Vec::new();
        node(&mut heap, -7, 0, Some(DIAGRAM as i16), &[]);
        label(&mut heap, Z_PLANE_LIST, b"#wirelint file-description");
        terminal(&mut heap, 0x0402_0040); // icon style, matches the confirmed sample bit
        terminal(&mut heap, 0x0002_0040); // data-type style, bit clear
        terminal(&mut heap, 0x0400_0000); // icon style, no other bits set
        node(&mut heap, -7, 2, None, &[]);
        let (bookmarks, terminal_icon_styles) = parse_heap(&heap).unwrap();
        assert_eq!(bookmarks, vec![b"#wirelint file-description".to_vec()]);
        assert_eq!(terminal_icon_styles, vec![true, false, true]);
    }

    #[test]
    fn ignores_obj_flags_outside_a_terminal_array_element() {
        let mut heap = Vec::new();
        node(&mut heap, -7, 0, Some(DIAGRAM as i16), &[]);
        node(
            &mut heap,
            OBJ_FLAGS,
            1,
            None,
            &ICON_STYLE_FLAG.to_be_bytes(),
        );
        node(&mut heap, ARRAY_ELEMENT, 0, Some(LABEL as i16), &[]);
        node(
            &mut heap,
            OBJ_FLAGS,
            1,
            None,
            &ICON_STYLE_FLAG.to_be_bytes(),
        );
        node(&mut heap, ARRAY_ELEMENT, 2, None, &[]);
        node(&mut heap, -7, 2, None, &[]);
        let (_, terminal_icon_styles) = parse_heap(&heap).unwrap();
        assert!(terminal_icon_styles.is_empty());
    }

    #[test]
    fn rejects_oversized_terminal_obj_flags() {
        let mut heap = Vec::new();
        node(&mut heap, ARRAY_ELEMENT, 0, Some(FP_TERM as i16), &[]);
        node(&mut heap, OBJ_FLAGS, 1, None, &[0, 0, 0, 0, 1]);
        node(&mut heap, ARRAY_ELEMENT, 2, None, &[]);
        assert!(parse_heap(&heap).is_err());
    }

    #[test]
    fn rejects_invalid_heap_scopes_and_lengths() {
        assert!(parse_heap(&[0x08, 24]).is_err());
        assert!(parse_heap(&[0x04, 24, 0x00]).is_err());
        assert!(parse_heap(&[0xa4, 24]).is_err());
    }
}
