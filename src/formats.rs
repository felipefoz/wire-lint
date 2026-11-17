//! Read-only extraction of analysis facts from LabVIEW documents.

mod rsrc;

use std::io;
use std::path::Path;

/// LabVIEW major and minor release numbers, ordered numerically.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Version {
    /// Parses a major.minor configuration value with representable components.
    pub fn parse(text: &str) -> Option<Self> {
        let (major, minor) = text.split_once('.')?;
        if major.is_empty()
            || minor.is_empty()
            || !major.bytes().all(|byte| byte.is_ascii_digit())
            || !minor.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let major = major.parse().ok()?;
        let minor = minor.parse().ok()?;
        (major <= 99 && minor <= 9).then_some(Self { major, minor })
    }

    /// Decodes the major and minor BCD digits of a LabVIEW version word.
    fn from_word(word: u32) -> io::Result<Self> {
        let tens = ((word >> 28) & 0xf) as u8;
        let ones = ((word >> 24) & 0xf) as u8;
        let minor = ((word >> 20) & 0xf) as u8;
        if tens > 9 || ones > 9 || minor > 9 {
            return Err(invalid("invalid LabVIEW version digits"));
        }
        Ok(Self {
            major: tens * 10 + ones,
            minor,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

/// Facts shared by all rules; extracted once for each analyzed file.
#[derive(Debug)]
pub struct FileFacts {
    pub separate_compiled_code: bool,
    pub saved_version: Option<Version>,
}

/// Reads a VI-family RSRC file or an XML project/library/class document.
///
/// Missing XML source-only properties mean false. When `read_version` is true,
/// missing or malformed version metadata is an error. Malformed documents and
/// unsupported binary layouts return `InvalidData`, never an assumed value.
pub fn read_facts(path: &Path, bytes: &[u8], read_version: bool) -> io::Result<FileFacts> {
    let xml = path.extension().and_then(|value| value.to_str());
    match xml {
        Some(extension) if extension.eq_ignore_ascii_case("lvproj") => {
            read_xml(bytes, "Project", read_version)
        }
        Some(extension) if extension.eq_ignore_ascii_case("lvlib") => {
            read_xml(bytes, "Library", read_version)
        }
        Some(extension) if extension.eq_ignore_ascii_case("lvclass") => {
            read_xml(bytes, "LVClass", read_version)
        }
        _ => rsrc::read_facts(bytes, read_version),
    }
}

/// Reads only the document owner's property, not nested items' properties.
fn read_xml(bytes: &[u8], expected_root: &str, read_version: bool) -> io::Result<FileFacts> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| invalid(format!("XML must be UTF-8: {error}")))?;
    let document = roxmltree::Document::parse(text.trim_start_matches('\u{feff}'))
        .map_err(|error| invalid(format!("invalid XML: {error}")))?;
    let root = document.root_element();
    if !root.has_tag_name(expected_root) {
        return Err(invalid(format!("expected XML root {expected_root}")));
    }

    let mut value = None;
    for property in root.children().filter(|node| {
        node.has_tag_name("Property") && node.attribute("Name") == Some("NI.LV.All.SourceOnly")
    }) {
        if value.is_some() {
            return Err(invalid("duplicate NI.LV.All.SourceOnly property"));
        }
        if property.attribute("Type") != Some("Bool")
            || property.children().any(|node| node.is_element())
        {
            return Err(invalid("NI.LV.All.SourceOnly must be a Bool property"));
        }
        let text: String = property
            .children()
            .filter_map(|node| node.text().filter(|_| node.is_text()))
            .collect();
        value = Some(match text.trim() {
            "true" => true,
            "false" => false,
            _ => return Err(invalid("NI.LV.All.SourceOnly must be true or false")),
        });
    }
    let saved_version = if read_version {
        let stamp = root
            .attribute("LVVersion")
            .ok_or_else(|| invalid("missing root LVVersion attribute"))?;
        if !(7..=8).contains(&stamp.len()) || !stamp.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("invalid root LVVersion attribute"));
        }
        let word = u32::from_str_radix(stamp, 16)
            .map_err(|_| invalid("invalid root LVVersion attribute"))?;
        Some(Version::from_word(word)?)
    } else {
        None
    };
    Ok(FileFacts {
        separate_compiled_code: value.unwrap_or(false),
        saved_version,
    })
}

/// Creates a contextual format error without substituting a setting value.
fn invalid(message: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_owner_xml_properties_for_all_xml_extensions() {
        for (extension, root) in [
            ("lvproj", "Project"),
            ("lvlib", "Library"),
            ("LVCLASS", "LVClass"),
        ] {
            for (property, expected) in [
                ("", false),
                (
                    r#"<Property Name="NI.LV.All.SourceOnly" Type="Bool">false</Property>"#,
                    false,
                ),
                (
                    r#"<Property Type="Bool" Name="NI.LV.All.SourceOnly"> true </Property>"#,
                    true,
                ),
            ] {
                let xml = format!("\u{feff}<?xml version=\"1.0\"?><{root}>{property}</{root}>");
                assert_eq!(
                    read_facts(
                        Path::new(&format!("file.{extension}")),
                        xml.as_bytes(),
                        false
                    )
                    .unwrap()
                    .separate_compiled_code,
                    expected
                );
            }
        }
    }

    #[test]
    fn ignores_nested_properties_comments_and_escaped_property_names() {
        let xml = br#"<Project>
            <!-- <Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property> -->
            <Item><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></Item>
        </Project>"#;
        assert!(
            !read_xml(xml, "Project", false)
                .unwrap()
                .separate_compiled_code
        );
        assert!(read_xml(
            br#"<Project><Property Name="NI.LV.All.Source&#79;nly" Type="Bool">true</Property></Project>"#,
            "Project", false
        ).unwrap().separate_compiled_code);
    }

    #[test]
    fn rejects_invalid_xml_values_and_duplicate_settings() {
        for xml in [
            "<Project>",
            "<Library/>",
            r#"<Project><Property Name="NI.LV.All.SourceOnly" Type="Str">true</Property></Project>"#,
            r#"<Project><Property Name="NI.LV.All.SourceOnly" Type="Bool">yes</Property></Project>"#,
            r#"<Project><Property Name="NI.LV.All.SourceOnly" Type="Bool"/></Project>"#,
            r#"<Project><Property Name="NI.LV.All.SourceOnly" Type="Bool">true<Item/></Property></Project>"#,
            r#"<Project><Property Name="NI.LV.All.SourceOnly" Type="Bool">false</Property><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></Project>"#,
        ] {
            assert!(read_xml(xml.as_bytes(), "Project", false).is_err(), "{xml}");
        }
    }

    #[test]
    fn reads_xml_version_as_a_hexadecimal_word() {
        for (stamp, major, minor) in [
            ("19008000", 19, 0),
            ("20008000", 20, 0),
            ("21308000", 21, 3),
            ("9008000", 9, 0),
        ] {
            let xml = format!(r#"<Project LVVersion="{stamp}"/>"#);
            assert_eq!(
                read_xml(xml.as_bytes(), "Project", true)
                    .unwrap()
                    .saved_version,
                Some(Version { major, minor })
            );
        }
        for xml in [
            "<Project/>",
            r#"<Project LVVersion=""/>"#,
            r#"<Project LVVersion="1"/>"#,
            r#"<Project LVVersion="0x20008000"/>"#,
            r#"<Project LVVersion="100000000"/>"#,
            r#"<Project LVVersion="2a008000"/>"#,
        ] {
            assert!(read_xml(xml.as_bytes(), "Project", true).is_err(), "{xml}");
            assert!(read_xml(xml.as_bytes(), "Project", false).is_ok(), "{xml}");
        }
    }
}
