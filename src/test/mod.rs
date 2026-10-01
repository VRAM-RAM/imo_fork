//! `imo`'s test suite.
//! \
//! **Safety:** the test suite works with single & multithreading.
//! \
//! You can run the test suite using simply `cargo test`.

use std::io::{BufRead, Write};

use crate::test::test_guard::TestOutputBase;

mod test_guard;


#[cfg(test)]
fn write_and_read(child: &mut std::process::Child, cmd: &str) -> String {
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = std::io::BufReader::new(stdout);

    writeln!(stdin, "{cmd}").expect("Failed to write to stdin");
    stdin.flush().unwrap();

    let mut response = String::new();
    reader
        .read_line(&mut response)
        .expect("Failed to read line");

    child.stdin = Some(stdin);
    child.stdout = Some(reader.into_inner());

    response
}

#[cfg(test)]
fn get_val(dbg_val: &str) -> &str {
    let (_, val) = dbg_val.split_once('=').unwrap();
    val.trim()
}

#[cfg(test)]
macro_rules! cmp {
    ($first:expr, $second:expr) => {
        let dbg_val = get_val($first);
        let raw_str = strip_ansi_escapes::strip_str(dbg_val);
        assert_eq!(raw_str, $second, "Values not equal");
    };
}

#[test]
fn boolean() {
    let base = TestOutputBase::new();

    base.write_to_output(&[], 
&[
            "let a = true;", 
            "let b = false;", 
            "let c = true;", 
            "let d = false;", 
            "let _p = false;"
            ]
    );

    let mut child = base.create_process();

    let _ = write_and_read(&mut child, "b 6");

    let _ = write_and_read(&mut child, "run");

    let a = write_and_read(&mut child, "p a");
    cmp!(&a, "true");

    let b = write_and_read(&mut child, "p b");
    cmp!(&b, "false");

    let c = write_and_read(&mut child, "p c");
    cmp!(&c, "true");

    let d = write_and_read(&mut child, "p d");
    cmp!(&d, "false");

    child.kill().unwrap();
}

#[test]
fn integer() {
    let base = TestOutputBase::new();

    base.write_to_output(&[], 
&[
            "let x:i32 = -15;", 
            "let p:i8 = -2;", 
            "let d:u64 = 12;", 
            "let e: usize = 13;", 
            "let _n = 500;" // Placeholder for a breakpoint to be placed
            ]
    );


    let mut child = base.create_process();

    let _ = write_and_read(&mut child, "b 6");

    let _ = write_and_read(&mut child, "run");

    let x = write_and_read(&mut child, "p x");
    cmp!(&x, "-15");

    let p = write_and_read(&mut child, "p p");
    cmp!(&p, "-2");

    let d = write_and_read(&mut child, "p d");
    cmp!(&d, "12");

    let e = write_and_read(&mut child, "p e");
    cmp!(&e, "13");

    child.kill().unwrap();
}

#[test]
fn char() {
    let base = TestOutputBase::new();

    base.write_to_output(&[], 
&[
            "let c = 'c';", 
            "let y = 'y';", 
            "let d = 'd';", 
            "let n = '2';", 
            "let _p = 'p';" 
            ]
    );

    let mut child = base.create_process();
    
    let _ = write_and_read(&mut child, "b 6");

    let _ = write_and_read(&mut child, "run");

    let c = write_and_read(&mut child, "p c");
    cmp!(&c, "'c'");

    let y = write_and_read(&mut child, "p y");
    cmp!(&y, "'y'");

    let d = write_and_read(&mut child, "p d");
    cmp!(&d, "'d'");

    let n = write_and_read(&mut child, "p n");
    cmp!(&n, "'2'");

    child.kill().unwrap();
}

#[test]
fn static_str() {
    let base = TestOutputBase::new();

    base.write_to_output(&[], 
&[
            "let foo = \"foo\";", 
            "let bar = \"bar\";", 
            "let baz = \"baz\";", 
            "let _p = \"Placeholder\";", 
            ]
    );

    let mut child = base.create_process();
    let _ = write_and_read(&mut child, "b 5");

    let _ = write_and_read(&mut child, "run");

    let foo = write_and_read(&mut child, "p foo");
    cmp!(&foo, "\"foo\"");

    let bar = write_and_read(&mut child, "p bar");
    cmp!(&bar, "\"bar\"");

    let baz = write_and_read(&mut child, "p baz");
    cmp!(&baz, "\"baz\"");

    child.kill().unwrap();
}

#[test]
fn string() {
    let base = TestOutputBase::new();

    base.write_to_output(&[], 
&[
            "let foo = String::from(\"foo\");", 
            "let bar = String::from(\"bar\");", 
            "let baz = String::from(\"baz\");", 
            "let _p = String::from(\"Placeholder\");", 
            ]
    );

    let mut child = base.create_process();
    
    let _ = write_and_read(&mut child, "b 5");

    let _ = write_and_read(&mut child, "run");

    let foo = write_and_read(&mut child, "p foo");
    cmp!(&foo, "\"foo\"");

    let bar = write_and_read(&mut child, "p bar");
    cmp!(&bar, "\"bar\"");

    let baz = write_and_read(&mut child, "p baz");
    cmp!(&baz, "\"baz\"");

    child.kill().unwrap();
}

#[test]
fn path() {
    let base = TestOutputBase::new();

    base.write_to_output(&["use std::path::Path;"], 
&[
            "let foo = Path::new(\"foo\");", 
            "let bar = Path::new(\"bar\");", 
            "let baz = Path::new(\"baz\");", 
            "let _p = Path::new(\"Placeholder\");", 
            ]
    );

    let mut child = base.create_process();
    
    let _ = write_and_read(&mut child, "b 6");

    let _ = write_and_read(&mut child, "run");

    let foo = write_and_read(&mut child, "p foo");
    cmp!(&foo, "Path(\"foo\")");

    let bar = write_and_read(&mut child, "p bar");
    cmp!(&bar, "Path(\"bar\")");

    let baz = write_and_read(&mut child, "p baz");
    cmp!(&baz, "Path(\"baz\")");

    child.kill().unwrap();
}

#[test]
fn path_buf() {
    let base = TestOutputBase::new();

    base.write_to_output(&["use std::path::PathBuf;"], 
&[
            "let foo = PathBuf::from(\"foo\");", 
            "let bar = PathBuf::from(\"bar\");", 
            "let baz = PathBuf::from(\"baz\");", 
            "let _p = PathBuf::from(\"Placeholder\");", 
            ]
    );

    let mut child = base.create_process();
    
    let _ = write_and_read(&mut child, "b 6");

    let _ = write_and_read(&mut child, "run");

    let foo = write_and_read(&mut child, "p foo");
    cmp!(&foo, "PathBuf(\"foo\")");

    let bar = write_and_read(&mut child, "p bar");
    cmp!(&bar, "PathBuf(\"bar\")");

    let baz = write_and_read(&mut child, "p baz");
    cmp!(&baz, "PathBuf(\"baz\")");

    child.kill().unwrap();
}
