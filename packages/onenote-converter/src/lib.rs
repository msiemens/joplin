mod fs;

use crate::fs::WasmFs;
use one2html::{Options, convert};
use std::panic;
use std::path::PathBuf;
use std::str::FromStr;
use wasm_bindgen::JsError;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn oneNoteConverter(input: &str, output: &str, base_path: &str) -> Result<(), JsError> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    let input = PathBuf::from_str(input)?;

    let base = input
        .parent()
        .ok_or_else(|| JsError::new("Couldn't find parent directory of input file"))?
        .strip_prefix(base_path)?;

    let output = PathBuf::from_str(output)?.join(base);

    if let Err(e) = convert(
        input.as_path(),
        output.as_path(),
        Options {
            warnings: true,
            math_target: one2html::MathTarget::LaTeX,
            note_tag_icons: one2html::NoteTagIcons::Emoji,
        },
        WasmFs {},
    ) {
        let message = format!("Error: {:?}", e);

        Err(JsError::new(&message))
    } else {
        Ok(())
    }
}
