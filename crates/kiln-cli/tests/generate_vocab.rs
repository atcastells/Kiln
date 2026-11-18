//! The vocabulary table and the Godot builder must not drift apart: the builder
//! reads every op and param by the exact string literal of the table.

use kiln_core::generate::vocab::OPS;

const BUILDER: &str = include_str!("../godot/kiln_generator.gd");
const RUNNER: &str = include_str!("../godot/generate.gd");

#[test]
fn builder_names_every_op_and_param_of_the_table() {
    for op in OPS {
        let quoted = format!("\"{}\"", op.id);
        assert!(BUILDER.contains(&quoted), "kiln_generator.gd lacks the op {quoted}");
        for p in op.params {
            let quoted = format!("\"{}\"", p.id);
            assert!(
                BUILDER.contains(&quoted),
                "kiln_generator.gd lacks the param {quoted} of {}",
                op.id
            );
        }
    }
}

#[test]
fn runner_loads_the_builder() {
    assert!(RUNNER.contains("kiln_generator.gd"));
}
