fileserver-rs
=============

Simple file server written in Rust.

![fileserver-rs](https://github.com/neevek/fileserver-rs/raw/master/screenshot/screenshot.jpg)

Build from source
-----------------

1. build the `frontend` crate:

  `frontend` will be built with the `trunk` program, which can be installed with `cargo install trunk`, see [trunkrs.dev](https://trunkrs.dev/) for details. 

  ```
  cd frontend
  trunk build --release --public-url '/assets'
  ```

  The `index.html` file and the generated wasm files will be copied to `frontend/dist`, these files will be used to render the frontend UI, to interact with the backend server.


2. build the `backend` crate:

  `cargo build --release --bin fileserver-rs`

  output of the `backend` crate is named `fileserver-rs`, which will be built as a command line app and located at `target/release/fileserver-rs`:


  ```
  A static file server that supports upload/create dir/delete/qrcode

  Usage: fileserver-rs [OPTIONS]

  Options:
    -l, --log <LOG_LEVEL>          Log level [default: debug]
    -a, --addr <ADDR>              Listen addr [default: 0.0.0.0]
    -p, --port <PORT>              Listen port [default: 8888]
        --assets-dir <ASSETS_DIR>  Directory where the wasm files built from the frontend sub crate are located [default: ./frontend/dist]
        --serve-dir <SERVE_DIR>    Directory to serve, default to the current directory if not specified [default: .]
    -h, --help                     Print help information

  ```

Run the server
--------------

Run the server to serve the `target` directory (this is for test):

`
./target/release/fileserver-rs -p 8888 --assets-dir ./frontend/dist --serve-dir ./target
`

Now the server is started at `http://localhost:8888/`.

MIT License
-------
Copyright 2022 Jiamin.Xie

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
