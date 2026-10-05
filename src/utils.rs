use owo_colors::OwoColorize;

/// Trim file path for the main source code
pub fn trim_file_path<P: AsRef<std::path::Path>>(path: P) -> String {
    path.as_ref()
        .file_name()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "main".to_string())
}

/// Display the code in a user friendly format
pub fn display_source_code(f: &mut std::fmt::Formatter<'_>, code: &str) -> std::fmt::Result {
    if code.starts_with("//") {
        // Displays the comments in gray
        write!(f, "{}", code.fg_rgb::<118, 118, 118>())?;
        return Ok(());
    }
    for token in code.split_whitespace() {
        match token {
            "let" | "fn" | "pub" | "impl" => write!(f, "{} ", token.red())?,
            "mut" | "return" | "if" => write!(f, "{} ", token.purple())?,
            "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "usize" | "isize"
            | "f32" | "f64" | "bool" | "char" => write!(f, "{} ", token.bright_yellow())?,
            _ => write!(f, "{token} ")?,
        }
    }
    Ok(())
}

pub fn display_help() {
    println!(r#"Hello World"#);
}
