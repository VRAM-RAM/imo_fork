use std::{fs::{self, File}, io::Write, path::{Path, PathBuf}};
use uuid::Uuid;


/// Private helpers that modifies a directory [`Path`] into a unique test directory [`PathBuf`].
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



pub struct TestOutputBase {
    dir: PathBuf,
}

impl TestOutputBase {
    pub fn new() -> Self {
        let test_path = std::env::current_dir().expect("Failed to retrieve current directory");
        let path = unique_test_path(&test_path);
        fs::create_dir_all(&path).expect("Failed to create the test directory");
        Self { dir: path }
    }

    pub fn write_to_output(&self, lib_decls: &[&str],inner_code: &[&str]) -> PathBuf {
        let rs_path = self.dir.join("out.rs");
                
        let mut rs_file = File::create(&rs_path).expect("Failed to create .rs test file");

        
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

        rs_path
    }

    pub fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Drop for TestOutputBase {
    fn drop(&mut self) {
        self.clear();
    }
}