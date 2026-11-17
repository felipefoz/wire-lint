//! Read-only extraction of analysis facts from LabVIEW documents.

mod rsrc;

use std::io;
use std::path::Path;

/// Facts shared by all rules; extracted once for each analyzed file.
#[derive(Debug)]
pub struct FileFacts {
    pub separate_compiled_code: bool,
}

/// Reads a VI-family RSRC file or an XML project/library/class document.
///
/// Missing XML properties mean false. Malformed documents and unsupported
/// binary layouts return `InvalidData`, never an assumed setting value.
pub fn read_facts(path: &Path, bytes: &[u8]) -> io::Result<FileFacts> {
    let xml = path.extension().and_then(|value| value.to_str());
    let separate_compiled_code = match xml {
        Some(extension) if extension.eq_ignore_ascii_case("lvproj") => read_xml(bytes, "Project")?,
        Some(extension) if extension.eq_ignore_ascii_case("lvlib") => read_xml(bytes, "Library")?,
        Some(extension) if extension.eq_ignore_ascii_case("lvclass") => read_xml(bytes, "LVClass")?,
        _ => rsrc::separate_compiled_code(bytes)?,
    };
    Ok(FileFacts {
        separate_compiled_code,
    })
}

/// Reads only the document owner's property, not nested items' properties.
fn read_xml(bytes: &[u8], expected_root: &str) -> io::Result<bool> {
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
    Ok(value.unwrap_or(false))
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
                    read_facts(Path::new(&format!("file.{extension}")), xml.as_bytes())
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
        assert!(!read_xml(xml, "Project").unwrap());
        assert!(read_xml(
            br#"<Project><Property Name="NI.LV.All.Source&#79;nly" Type="Bool">true</Property></Project>"#,
            "Project"
        ).unwrap());
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
            assert!(read_xml(xml.as_bytes(), "Project").is_err(), "{xml}");
        }
    }
}
