use crate::test::{test_base::TestOutputBase, write_and_read};


#[macro_export]
#[cfg(test)]
macro_rules! cmp_regex {
    ($first:expr, $pattern:expr) => {{
        let raw_str = strip_ansi_escapes::strip_str($first);
        let re = regex_lite::Regex::new($pattern).unwrap();
        assert!(re.is_match(&raw_str), "Pattern '{}' not found in '{}'", $pattern, raw_str);
    }};
}

#[test]
/// Tests a simple watchpoint case.
fn watchpoint_simple() {
    let mut base = TestOutputBase::new();

    base.write_to_output(&[], 
        &[
            "let mut x = 3;",      
            "x += 2;"             
        ]
    );

    let index = base.create_process();

    let child = base.child_as_mut_ref(index).unwrap();

    write_and_read(child, "b 3");
    
    let _run = write_and_read(child, "run");

    let wp_creation = write_and_read(child, "w x:2");
    
    println!("Watchpoint creation output : {}", wp_creation);

    cmp_regex!(&wp_creation, r"Watchpoint \d+ at .*variable x at line 2");

    let wp_triggered = write_and_read(child, "c");

    println!("Watchpoint change output : {}", wp_triggered);

    // Vérifier que la watchpoint a bien été déclenchée avec le bon changement de valeur (3 -> 5)
    cmp_regex!(&wp_triggered, r"Variable x .*changed from 3 to 5");
}