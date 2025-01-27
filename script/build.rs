use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    // This will be compiled inside a docker container already
    build_program_with_args(
        "../program",
        BuildArgs {
            tag: "v4.0.1".to_string(),
            ..Default::default()
        },
    )
}
