//! Implements [`TestOutputBase`], a structure made for isolating tests file writing, and so enable multithreading when testing.

use std::{fs::{self, File}, io::Write, path::{Path, PathBuf}, process::Child};
use uuid::Uuid;


/// Turns a directory [`Path`] into a unique test directory [`PathBuf`].
/// \
/// It first join `/src/test/out/` to it, then generate an unique ID, joined to the path.
/// \
/// So it returns, for example :
/// ```text
/// /home/user/rust/imo/src/test/out/67e55044-10b1-426f-9247-bb680e5fe0c8/
/// ```
fn unique_test_path(path: &Path) -> PathBuf {
    let path = path.join("src").join("test").join("out");
    let id = Uuid::new_v4().to_string();
    path.join(Path::new(&id))
}

/// A structure made for isolating tests file writing. 
/// \
/// Every test gets an unique path, source (`.rs`) file and binary file, so that the tests can be executed in parallel.
/// \
/// When dropped, [`TestOutputBase`] automatically suppresses all the artifacts it generated.
/// \
/// Example : 
/// ```rust
/// fn test_base() {
///     let base = TestOutputBase::new(); // Creates a new unique test directory, and all the associated paths
///     base.write_to_output(&[], &[
///         "let base = TestOutputBase::new();",
///         "base.clear();"
///     ]);
///     
///     let mut child = base.create_process();
///     child.kill().unwrap();
/// } // When exiting the scope, and so dropping the `base`, the unique test directory is cleared.
/// ```
pub struct TestOutputBase {
    /// The generated unique test path (see [unique_test_path] for more informations)
    dir: PathBuf,

    /// The path to the `out.rs` source file
    source_path: PathBuf,

    /// The path to the `out` binary file
    binary_path: PathBuf,
}

#[allow(dead_code)]
impl TestOutputBase {

    /// Returns a new [`TestOutputBase`].
    /// \
    /// It retrieves the current working directory, and convert it into a unique test path using [`unique_test_path`].
    /// \
    /// Then, creates the test directory, and the **source** and **binary** paths.
    pub fn new() -> Self {
        let test_path = std::env::current_dir().expect("Failed to retrieve current directory");
        let path = unique_test_path(&test_path);
        fs::create_dir_all(&path).expect("Failed to create the test directory");
        let rs_path = path.join("out.rs");
        let bin_path = path.join("out");
        Self { dir: path, source_path: rs_path, binary_path: bin_path  }
    }

    /// Writes `rust` source code to the `out.rs` file.
    /// \
    /// The `lib_decls` (library declarations) are written at the top of the file. Each `&str` represents a line of code.
    /// \
    /// The `inner_code` is written in a `main()` function. Each `&str` represents a line of code too.
    /// \
    /// Example :
    /// ```rust
    /// let base = TestOutputBase::new();
    /// base.write_to_output(&["use foo::display::foo_disp;"]
    ///     &["let foo: String = String::from(\"foo\");"],
    ///     ["let display_result = foo_disp(foo);"],
    ///     ["if display_result.is_err() { eprintln!(\"Error while displaying foo!\"); }"]
    /// );
    /// ```
    /// \
    /// Will write :
    /// ```rust
    /// use foo::display::foo_disp;
    /// 
    /// fn main() {
    /// let foo: String = String::from("foo");
    /// let display_result = foo_disp(foo);
    /// if display_result.is_err() { eprintln!("Error while displaying foo!"); }
    /// }
    /// ```
    /// The problem of this approach is that we directly pass `&str`s, and not `rust` tokens. So, for example, be aware when writing strings (don't forget : `\"` instead of `"`).
    /// 
    pub fn write_to_output(&self, lib_decls: &[&str],inner_code: &[&str]) {      
        let mut rs_file = File::create(&self.source_path).expect("Failed to create .rs test file");

        for lib_decl in lib_decls {
            rs_file.write(lib_decl.as_bytes()).unwrap();
            rs_file.write(b"\n").unwrap();
        }

        rs_file.write(b"fn main() {\n").unwrap();
        
        for code in inner_code {
            rs_file.write(code.as_bytes()).unwrap();
            rs_file.write(b"\n").unwrap();
        }

        rs_file.write(b"}\n").unwrap();
    }

    /// Returns a reference to the unique test path.
    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    /// Returns a reference to the source path.
    pub fn src_path(&self) -> &PathBuf {
        &self.source_path
    }

    /// Returns a reference to the binary path.
    pub fn binary_path(&self) -> &PathBuf {
        &self.binary_path
    }

    /// Compiles the `out.rs` source file into the `out` binary file, calling `rustc`.
    pub fn compile(&self) {
        std::process::Command::new("rustc")
        .arg("-g")
        .arg("-o")
        .arg(&self.binary_path)
        .arg(&self.source_path)
        .output()
        .expect("Failed to compile code");
    }

    /// Compiles the `out.rs` source file calling [`TestOutputBase::compile`], and runs the debugger.
    pub fn create_process(&self) -> Child {
        self.compile();

        let child = std::process::Command::new("cargo")
            .arg("run")
            .arg(&self.binary_path.display().to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to start debugger");

        child
    }

    /// Clears the test directory. Prints an error message if removing fails (when `--nocapture` is enabled).
    /// \
    /// Warning : the unique test directory and its content are deleted, but not the root `/test/out/` directory.
    pub fn clear(&self) {
        if let Err(e) = std::fs::remove_dir_all(&self.dir) {
            eprintln!("Error while removing the directory : {}", e)
        }
    }
}

impl Drop for TestOutputBase {
    /// Clears the test directory on drop.
    fn drop(&mut self) {
        self.clear();
    }
}