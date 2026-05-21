mod fs;

use crate::fs::WasmFs;
use one2html::{convert, Options};
use onenote_parser::FileSystem;
use std::panic;
use typed_path::{TypedPathBuf, UnixPath, WindowsPath};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn oneNoteConverter(input: &str, output: &str, base_path: &str) -> Result<(), JsError> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    let fs = WasmFs {};

    let input = to_path(input, fs);

    let parent = input
        .parent()
        .ok_or_else(|| JsError::new("Couldn't find parent directory of input file"))?;
    let base = parent.strip_prefix(base_path)?;

    let output = to_path(output, fs).join(base);

    if let Err(e) = convert(
        input.to_path(),
        output.to_path(),
        Options {
            warnings: true,
            math_target: one2html::MathTarget::LaTeX,
            note_tag_icons: one2html::NoteTagIcons::Emoji,
        },
        fs,
    ) {
        let message = format!("Error: {:?}", e);

        Err(JsError::new(&message))
    } else {
        Ok(())
    }
}

fn to_path(str: &str, fs: WasmFs) -> TypedPathBuf {
    if fs.is_windows() {
        WindowsPath::new(str).to_typed_path_buf()
    } else {
        UnixPath::new(str).to_typed_path_buf()
    }
}
