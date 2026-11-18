//! The vocabulary table and the Godot script must not drift apart: the script
//! reads every op and param by the exact string literal of the table.

use kiln_core::generate::vocab::OPS;

const SCRIPT: &str = include_str!("../godot/generate.gd");

#[test]
fn script_names_every_op_and_param_of_the_table() {
    for op in OPS {
        let quoted = format!("\"{}\"", op.id);
        assert!(SCRIPT.contains(&quoted), "generate.gd lacks the op {quoted}");
        for p in op.params {
            let quoted = format!("\"{}\"", p.id);
            assert!(
                SCRIPT.contains(&quoted),
                "generate.gd lacks the param {quoted} of {}",
                op.id
            );
        }
    }
}
