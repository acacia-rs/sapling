//! A pack's `manifest.json`: who it is (uuid, version) and what it holds (its modules' types).

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub name: String,
    /// Lowercase, as clients echo it back.
    pub uuid: String,
    /// `1.0.0`: the manifest's `[1, 0, 0]` or its string.
    pub version: String,
    /// `resources`, `data`, `script`, `client_data`, ...
    pub modules: Vec<String>,
}

impl Manifest {
    pub fn parse(json: &[u8]) -> Result<Self, String> {
        // Packs are written by hand: a byte-order mark is common.
        let json = json.strip_prefix(b"\xef\xbb\xbf").unwrap_or(json);
        let root: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
        let header = root.get("header").ok_or("no header")?;
        let uuid = header.get("uuid").and_then(Value::as_str).ok_or("no header.uuid")?.to_ascii_lowercase();
        if uuid.len() != 36 || uuid.split('-').map(str::len).ne([8, 4, 4, 4, 12]) || !uuid.chars().all(|c| c == '-' || c.is_ascii_hexdigit()) {
            return Err(format!("header.uuid {uuid:?} is not a uuid"));
        }
        let version = version(header.get("version")).ok_or("no header.version")?;
        let name = header.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
        let modules = root.get("modules").and_then(Value::as_array).into_iter().flatten().filter_map(|module| module.get("type")?.as_str().map(str::to_owned)).collect();
        Ok(Self { name, uuid, version, modules })
    }

    /// The id clients name the pack by in ResourcePackClientResponse: `<uuid>_<version>`.
    pub fn id(&self) -> String {
        format!("{}_{}", self.uuid, self.version)
    }

    pub fn has_module(&self, kind: &str) -> bool {
        self.modules.iter().any(|module| module == kind)
    }
}

/// A version as `[1, 0, 0]` or `"1.0.0"`, written `1.0.0`.
pub(crate) fn version(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Array(parts) => parts.iter().map(|part| part.as_u64().map(|n| n.to_string())).collect::<Option<Vec<_>>>().map(|parts| parts.join(".")),
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_names_its_pack() {
        let json = br#"{"format_version":2,"header":{"name":"probe","uuid":"5D9A0A52-5d2f-4b47-9a43-34c0e2b1f001","version":[1,2,3]},"modules":[{"type":"resources","uuid":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f002","version":[1,0,0]}]}"#;
        let manifest = Manifest::parse(json).unwrap();
        assert_eq!((manifest.id().as_str(), manifest.name.as_str()), ("5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001_1.2.3", "probe"));
        assert!(manifest.has_module("resources") && !manifest.has_module("data"));
        let text_version = br#"{"header":{"uuid":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001","version":"2.0.0"}}"#;
        assert_eq!(Manifest::parse(text_version).unwrap().version, "2.0.0");
        assert!(Manifest::parse(br#"{"header":{"uuid":"nope","version":[1,0,0]}}"#).is_err());
        assert!(Manifest::parse(br#"{"header":{"uuid":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001"}}"#).is_err());
    }
}
