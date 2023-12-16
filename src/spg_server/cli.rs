use std::{env, env::VarError};

/// There's no real CLI for the server, so just do quick 'n dirty
pub fn handle_command_line_args() {
    if env::args().count() > 1 {
        // We don't expect any CLI args, so always print the help
        display_readme();
        display_envs();
    }
}

fn display_readme() {
    const README: &str = include_str!("../cli-help.txt");
    println!("\n{README}\n");
}

fn display_envs() {
    // Be explicit about which envars to print, so as to avoid accidentally exposing secrets
    const DISPLAY_ENVS: [&str; 1] = ["RUST_LOG"];

    println!("Current environment values:");
    DISPLAY_ENVS.iter().for_each(|&name| {
        let val = match env::var(name) {
            Ok(s) => s,
            Err(VarError::NotPresent) => "Not set".into(),
            Err(VarError::NotUnicode(s)) => format!("Invalid value: {}", s.to_string_lossy()),
        };
        println!("  {name:<35} {val:<15}");
    })
}
