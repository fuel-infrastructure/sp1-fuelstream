//! Automatically build program (generate elf) whenever a script is ran.
use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    let args = BuildArgs {
        docker: true,
        elf_name: "fuelstreamx-elf".to_string(),
        ..Default::default()
    };
    build_program_with_args("../program", &args);
}
