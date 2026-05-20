use bytes::Bytes;
use js_sys::Array;
use onenote_parser::FileSystem;
use onenote_parser::fs::{FileSource, CachedFileSource};
use std::io::{Error, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::js_sys::Uint8Array;

#[wasm_bindgen(module = "/node_functions.js")]
extern "C" {
    #[wasm_bindgen(js_name = makeDir, catch)]
    fn make_dir(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = normalizeAndWriteFile, catch)]
    fn write_file(path: &str, data: &[u8]) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = isDirectory, catch)]
    fn is_directory(path: &str) -> Result<bool, JsValue>;

    #[wasm_bindgen(js_name = readDir, catch)]
    fn read_dir(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = openFileForReading, catch)]
    fn open_file_for_reading(path: &str) -> Result<i32, JsValue>;

    #[wasm_bindgen(js_name = openFileForWriting, catch)]
    fn open_file_for_writing(path: &str) -> Result<i32, JsValue>;

    #[wasm_bindgen(js_name = writeChunk, catch)]
    fn write_chunk(fd: i32, data: &[u8]) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = closeFile, catch)]
    fn close_file(fd: i32) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = fileSize, catch)]
    fn file_size(path: &str) -> Result<f64, JsValue>;

    #[wasm_bindgen(js_name = readFileChunk, catch)]
    fn read_file_chunk(fd: i32, offset: f64, length: u32) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(module = "fs")]
extern "C" {
    #[wasm_bindgen(js_name = readFileSync, catch)]
    fn read_file(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = existsSync, catch)]
    fn exists(path: &str) -> Result<bool, JsValue>;
}

#[derive(Copy, Clone)]
pub(crate) struct WasmFs {}

impl FileSystem for WasmFs {
    fn is_directory(&self, path: &Path) -> Result<bool, Error> {
        is_directory(path.to_string_lossy().as_ref())
            .map_err(|e| handle_error(e, "checking is_directory"))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
        let value = read_dir(path.to_string_lossy().as_ref())
            .map_err(|e| handle_error(e, "reading dir"))?;

        let arr = Array::from(&value);

        let mut out = Vec::with_capacity(arr.length() as usize);
        for i in 0..arr.length() {
            if let Some(s) = arr.get(i).as_string() {
                out.push(PathBuf::from(s));
            }
        }

        Ok(out)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, Error> {
        let path = path.to_string_lossy();
        let value = read_file(path.as_ref())
            .map_err(|e| handle_error(e, &format!("reading file {}", path)))?;
        Ok(Uint8Array::new(&value).to_vec())
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), Error> {
        let path = path.to_string_lossy();
        write_file(path.as_ref(), data)
            .map(|_| ())
            .map_err(|e| handle_error(e, &format!("writing file {}", path)))
    }

    fn stream_to_file(&self, path: &Path, reader: &mut dyn Read) -> Result<(), Error> {
        let path_str = path.to_string_lossy();

        // openSync(path, 'w') creates and truncates, so zero-byte streams still
        // produce a file. Keep the fd open across chunks to avoid an
        // open/write/close cycle per chunk; the WriteFd guard closes it on scope
        // exit (success or error).
        let guard = WriteFd(
            open_file_for_writing(path_str.as_ref())
                .map_err(|e| handle_error(e, &format!("opening file {} for writing", path_str)))?,
        );

        let mut chunk_size: usize = 1024 * 1024;
        let max_chunk_size: usize = 50 * 1024 * 1024;
        let mut buffer = vec![0u8; chunk_size];

        loop {
            let size = match reader.read(&mut buffer) {
                Ok(size) => size,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            };

            if size == 0 {
                break;
            }

            write_chunk(guard.0, &buffer[0..size])
                .map_err(|e| handle_error(e, &format!("writing file {}", path_str)))?;

            if size == chunk_size && chunk_size < max_chunk_size {
                chunk_size = (chunk_size * 2).min(max_chunk_size);
                buffer.resize(chunk_size, 0);
            }
        }

        Ok(())
    }

    fn open_file(&self, path: &Path) -> Result<Arc<dyn FileSource>, Error> {
        let path_str = path.to_string_lossy();

        let byte_length = file_size(path_str.as_ref())
            .map_err(|e| handle_error(e, &format!("stat {}", path_str)))?
            as u64;

        let fd = open_file_for_reading(path_str.as_ref())
            .map_err(|e| handle_error(e, &format!("open {}", path_str)))?;

        Ok(Arc::new(CachedFileSource::new(NodeFileSource { fd, byte_length })))
    }

    fn make_dir(&self, path: &Path) -> Result<(), Error> {
        let path = path.to_string_lossy();
        make_dir(path.as_ref())
            .map(|_| ())
            .map_err(|e| handle_error(e, &format!("mkdir {}", path)))
    }

    fn exists(&self, path: &Path) -> Result<bool, Error> {
        let path = path.to_string_lossy();
        exists(path.as_ref()).map_err(|e| handle_error(e, &format!("checking exists {}", path)))
    }
}

/// RAII guard that closes a Node.js write fd on scope exit.
struct WriteFd(i32);

impl Drop for WriteFd {
    fn drop(&mut self) {
        let _ = close_file(self.0);
    }
}

/// A [`FileSource`] backed by an open Node.js file descriptor.
///
/// Reads are issued via `fs.readSync` on demand, so the file's bytes never need
/// to be materialised in WASM memory in full. Closes the fd on drop.
struct NodeFileSource {
    fd: i32,
    byte_length: u64,
}

// WASM is single-threaded; the fd is just an integer handle into Node-side state,
// and the trait bound `FileSource: Send + Sync` is required by the parser.
unsafe impl Send for NodeFileSource {}
unsafe impl Sync for NodeFileSource {}

impl Drop for NodeFileSource {
    fn drop(&mut self) {
        let _ = close_file(self.fd);
    }
}

impl FileSource for NodeFileSource {
    fn byte_length(&self) -> u64 {
        self.byte_length
    }

    fn read_at(&self, offset: u64, len: usize) -> Result<Bytes, Error> {
        let value = read_file_chunk(self.fd, offset as f64, len as u32)
            .map_err(|e| handle_error(e, &format!("reading {} bytes at offset {}", len, offset)))?;
        let chunk = Uint8Array::new(&value).to_vec();
        Ok(Bytes::from(chunk))
    }
}

fn handle_error(error: JsValue, source: &str) -> Error {
    let message = js_sys::Error::from(error)
        .message()
        .as_string()
        .unwrap_or_default();
    Error::other(format!("{}: {}", source, message))
}
