//! `multipart/form-data` bodies: text fields and files, encoded to bytes up front so a retried
//! request sends the same body again.

use std::{
    fmt::Write as _,
    io,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
};

/// A `multipart/form-data` body (an HTML form with file inputs), for
/// [`Options::multipart`](super::Options::multipart):
///
/// ```
/// use rok_ui::http::{Multipart, Options};
///
/// let form = Multipart::new()
///     .text("title", "Holiday")
///     .bytes("photo", "beach.png", "image/png", vec![0x89, b'P', b'N', b'G']);
/// let options = Options::post().multipart(form);
/// # let _ = options;
/// ```
#[derive(Clone, Debug, Default)]
pub struct Multipart {
    parts: Vec<Part>,
}

/// One field of a [`Multipart`] body.
#[derive(Clone, Debug)]
struct Part {
    name: String,
    /// The file name and content type, for a file.
    file: Option<(String, String)>,
    data: Vec<u8>,
}

impl Multipart {
    /// An empty form.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a text field.
    #[must_use]
    pub fn text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parts.push(Part {
            name: name.into(),
            file: None,
            data: value.into().into_bytes(),
        });
        self
    }

    /// Add a file from memory, with the name and content type the server sees.
    #[must_use]
    pub fn bytes(
        mut self,
        name: impl Into<String>,
        file_name: impl Into<String>,
        content_type: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        self.parts.push(Part {
            name: name.into(),
            file: Some((file_name.into(), content_type.into())),
            data: data.into(),
        });
        self
    }

    /// Add the file at `path`, read now (in a provider, which runs off the UI thread). Its
    /// content type comes from the extension (see [`content_type_for`]); its file name is the
    /// path's last component.
    ///
    /// # Errors
    ///
    /// The file could not be read.
    pub fn file(self, name: impl Into<String>, path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        let file_name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        Ok(self.bytes(name, file_name, content_type_for(path), data))
    }

    /// The body and its `Content-Type` (with the boundary).
    pub(super) fn encode(&self) -> (Vec<u8>, String) {
        let boundary = self.boundary();
        let mut body = Vec::new();
        for part in &self.parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            let mut disposition = format!("form-data; name=\"{}\"", escape(&part.name));
            if let Some((file_name, _)) = &part.file {
                write!(disposition, "; filename=\"{}\"", escape(file_name)).ok();
            }
            body.extend_from_slice(format!("Content-Disposition: {disposition}\r\n").as_bytes());
            if let Some((_, content_type)) = &part.file {
                let content_type = content_type.replace(['\r', '\n'], "");
                body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
            }
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(&part.data);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        (body, format!("multipart/form-data; boundary={boundary}"))
    }

    /// A boundary that appears in no part.
    fn boundary(&self) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        static SEED: OnceLock<u64> = OnceLock::new();
        let seed = *SEED.get_or_init(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos() as u64)
        });
        loop {
            let boundary = format!(
                "rok-ui-{seed:016x}-{:04x}",
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let clashes = self.parts.iter().any(|part| {
                part.data
                    .windows(boundary.len())
                    .any(|window| window == boundary.as_bytes())
            });
            if !clashes {
                return boundary;
            }
        }
    }
}

/// A name or file name for a `Content-Disposition` header: quotes and line breaks escaped
/// the way browsers do.
fn escape(text: &str) -> String {
    text.replace('"', "%22")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// The content type for a file, from its extension: common images, documents, audio and video,
/// else `application/octet-stream`.
#[must_use]
pub fn content_type_for(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_escaped_and_boundaries_avoid_the_content() {
        let form = Multipart::new().text("a\"b\r\nc", "x");
        let (body, _) = form.encode();
        assert!(String::from_utf8(body)
            .unwrap()
            .contains("name=\"a%22b%0D%0Ac\""));

        // A part that holds the next boundary makes the encoder pick another one.
        let (_, content_type) = Multipart::new().encode();
        let used = content_type.rsplit('=').next().unwrap().to_string();
        let (prefix, counter) = used.rsplit_once('-').unwrap();
        let next = u64::from_str_radix(counter, 16).unwrap() + 1;
        let tricky = format!("{prefix}-{next:04x}");
        let (body, content_type) = Multipart::new().text("t", &tricky).encode();
        let boundary = content_type.rsplit('=').next().unwrap();
        assert!(String::from_utf8(body).unwrap().contains(&tricky));
        assert_ne!(boundary, tricky);
    }

    #[test]
    fn files_are_read_with_their_name_and_type() {
        let path =
            std::env::temp_dir().join(format!("rok-ui-multipart-{}.csv", std::process::id()));
        std::fs::write(&path, "a,b\n").unwrap();
        let (body, _) = Multipart::new().file("data", &path).unwrap().encode();
        std::fs::remove_file(&path).ok();
        let body = String::from_utf8(body).unwrap();
        let file_name = path.file_name().unwrap().to_string_lossy();
        assert!(body.contains(&format!("name=\"data\"; filename=\"{file_name}\"\r\n")));
        assert!(body.contains("Content-Type: text/csv\r\n\r\na,b\n\r\n"));
        assert!(Multipart::new().file("missing", "/no/such/file").is_err());
    }

    #[test]
    fn content_types_cannot_add_headers() {
        let (body, _) = Multipart::new()
            .bytes("f", "a.txt", "text/plain\r\nX-Injected: 1", b"x".to_vec())
            .encode();
        let body = String::from_utf8(body).unwrap();
        assert!(body.contains("Content-Type: text/plainX-Injected: 1\r\n"));
        assert!(!body.contains("\r\nX-Injected"));
    }

    #[test]
    fn content_types_follow_the_extension() {
        assert_eq!(content_type_for(Path::new("a/photo.JPG")), "image/jpeg");
        assert_eq!(content_type_for(Path::new("notes.txt")), "text/plain");
        assert_eq!(
            content_type_for(Path::new("archive")),
            "application/octet-stream"
        );
    }
}
