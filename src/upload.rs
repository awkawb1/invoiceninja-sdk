use std::fmt;

use reqwest::multipart::Part;

use crate::error::{Error, Result};

/// A file to attach to a record with the `upload_*` methods.
///
/// The contents are held in memory, so a large file costs its full size in
/// RAM for as long as the upload takes. Streaming uploads are an open
/// question in `GOALS.md`.
///
/// ```
/// use invoiceninja_sdk::FileUpload;
///
/// let file = FileUpload::new("receipt.pdf", b"%PDF-1.4 ...".to_vec())
///     .mime("application/pdf");
/// ```
#[derive(Clone)]
pub struct FileUpload {
    file_name: String,
    bytes: Vec<u8>,
    mime: Option<String>,
}

impl FileUpload {
    /// `file_name` is what the server records and shows against the
    /// document, so it should carry the right extension.
    pub fn new(file_name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            file_name: file_name.into(),
            bytes: bytes.into(),
            mime: None,
        }
    }

    /// Sets the part's `Content-Type`. Optional: without it the part carries
    /// none, and the server decides from the file's contents.
    pub fn mime(mut self, mime: impl Into<String>) -> Self {
        self.mime = Some(mime.into());
        self
    }

    pub(crate) fn into_part(self) -> Result<Part> {
        if self.file_name.trim().is_empty() {
            return Err(Error::InvalidUpload(
                "a file needs a non-empty file name".into(),
            ));
        }

        let part = Part::bytes(self.bytes).file_name(self.file_name);
        match self.mime {
            Some(mime) => part
                .mime_str(&mime)
                .map_err(|_| Error::InvalidUpload(format!("`{mime}` is not a valid MIME type"))),
            None => Ok(part),
        }
    }
}

/// Elides the contents: the derived impl would print the entire file.
impl fmt::Debug for FileUpload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileUpload")
            .field("file_name", &self.file_name)
            .field("len", &self.bytes.len())
            .field("mime", &self.mime)
            .finish()
    }
}
