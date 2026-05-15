# OneNote Converter

This package is used to process OneNote backup files and output HTML that Joplin can import.

The code is based on the [`onenote_parser`](https://github.com/msiemens/onenote.rs) and [`one2html`](https://github.com/msiemens/one2html) crates created by https://github.com/msiemens.

This package is a thin WebAssembly bridge over those crates. All actual parsing and rendering logic lives in them; here we wire them up to wasm-bindgen and provide a Node.js filesystem implementation so the renderer can read input and write output via Node's `fs` module (see `node_functions.js` and `src/fs.rs`).

## How the OneNote Importer Process Works

The requirement for this project was to simplify the migration process from OneNote to Joplin. The starting point of this migration is to export the notebook from OneNote as a `zip` file containing files in the binary format used by OneNote.

The process looks like this:

1. Unzip the backup file.
2. Use `onenote-converter` to read and convert the binary files to HTML (this project).
3. Extract the SVG nodes from the HTML to resources:
    1. Find all SVG nodes in the HTML file.
    2. Create SVG files from the nodes.
    3. Update the HTML file with references to the SVGs.
4. Rewrite `<embed>` / `<audio>` / `<video>` references emitted by the renderer into anchors so the Markdown importer can attach them as Joplin resources.
5. Use the Importer HTML service to create the Joplin notes and resources.

See the `InteropService_Importer_OneNote` class in the `lib` project for details.

### SVG Extraction

The OneNote drawing feature uses `<svg>` tags to save user drawings. Joplin doesn't support SVG rendering due to security concerns, so we added a step to extract the `<svg>` elements as SVG images, replacing them with `<img>` tags.

For each HTML file, we:

- Mount the HTML in the document.
- Find all the `svg` nodes.
- Replace each `svg` node with an `img` node that has a unique title, which will be used as the resource name.
- After editing the entire document, update the HTML.
- Create the SVG images on the local disk with the title used in the replaced `img` tags.

After this, the HTML should look the same and is ready to be imported by the Importer HTML service.

## Project structure:

```
- onenote-converter
    - package.json              -> where the project is built
    - Cargo.toml                -> single-crate manifest; depends on onenote_parser + one2html
    - node_functions.js         -> where the custom-made functions used inside rust goes
    ...
    - pkg                       -> artifact folder generated in the build step
        - joplin_interop.js     -> main file
    ...
    - src
        - lib.rs                -> #[wasm_bindgen] entry point
        - fs.rs                 -> WasmFs: implements onenote_parser::FileSystem on top of node_functions.js
```

## Development requirements:

To work with the project you will need:

- Rust https://www.rust-lang.org/learn/get-started

### Building

For most setups, the OneNote converter must be built manually:
- Development build: `yarn buildDev`.
    - Includes additional logging.
    - Faster compilation.
    - Slower at runtime.
- Production build: `IS_CONTINUOUS_INTEGRATION=1 yarn build`
    - **Important**: The `IS_CONTINUOUS_INTEGRATION` environment variable must be set. To simplify the development process for contributors without Rust installed, `yarn build` is disabled unless `IS_CONTINUOUS_INTEGRATION` is set.

### Running tests

Most tests for the project are located in the `lib` packages, but to make it work it is necessary to build this project first:

`IS_CONTINUOUS_INTEGRATION=1 yarn build # for production build`
or 
`IS_CONTINUOUS_INTEGRATION=1 yarn buildDev # for build with more logs and compiles faster`

After that you should navigate to `lib` package and run the tests of `InteropService_Importer_OneNote.test.` file

```
cd ../lib
IS_CONTINUOUS_INTEGRATION=1 yarn test services/interop/InteropService_Importer_OneNote.test.
```

Other tests are written in Rust. To run these tests, use `cargo test`:
```
cd packages/onenote-converter
cargo test
```

The bulk of the unit tests for parsing and rendering live in the upstream `onenote_parser` and `one2html` crates respectively. Bugs in OneNote parsing or HTML output are usually best reproduced and fixed there.

### Debugging tests

Suppose that the importer's Rust code is failing to parse a specific `example.one` file. In this case, it may be useful to step through part of the import process in a debugger. If using VSCode, this can be done by:
1. Adding a new test in the upstream `one2html` crate that runs `convert()` on the `example.one` file.
2. Setting up Rust and Rust debugging. See [the relevant VSCode documentation](https://code.visualstudio.com/docs/languages/rust#_debugging) for details.
3. Clicking the "Debug" button for the test added in step 1. This button should be provided by extensions set up in step 2.



### Developing

When working with the Rust code you will probably rather run `yarn buildDev` since it is faster and it has more logging messages (they can be disabled in the macro `log!()`)

During development, it will be easier to test it where this library is called. `InteropService_Importer_Onenote.ts` is the code that depends on this and already has some tests.

We don't require developers that won't work on this project to have Rust installed on their machine.
To make this work we:

- Use temporary files, required only for building the application correctly (e.g: `pkg/joplin_interop.js`).
- Skip the build process if `IS_CONTINUOUS_INTEGRATION` is not set (see `tools/build.js`).
- Skip some tests if `IS_CONTINUOUS_INTEGRATION` is not set (see `lib/services/interop/InteropService_Importer_OneNote.test.ts`).

The tests should still run on CI since `IS_CONTINUOUS_INTEGRATION` is used there.

## Security concerns

We are using WebAssembly with Node.js calls to the file system, reading and writing files and directories, which means
it is not isolated (no more than Node.js is, for that matter). 
