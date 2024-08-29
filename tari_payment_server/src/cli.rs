use std::{env, env::VarError};

/// There's no real CLI for the server, so just do quick 'n dirty
pub fn handle_command_line_args() -> bool {
    let has_cli_args = env::args().count() > 1;
    if has_cli_args {
        // We don't expect any CLI args, so always print the help
        display_readme();
        display_envs();
    }
    has_cli_args
}

fn display_readme() {
    const README: &str = include_str!("./cli-help.txt");
    println!("\n{README}\n");
}

fn display_envs() {
    // Be explicit about which envars to print, so as to avoid accidentally exposing secrets
    const DISPLAY_ENVS: [&str; 14] = [
        "RUST_LOG",
        "TPG_SHOPIFY_SHOP",
        "TPG_SHOPIFY_API_VERSION",
        "TPG_SHOPIFY_HMAC_CHECKS",
        "TPG_HOST",
        "TPG_PORT",
        "TPG_DATABASE_URL",
        "TPG_SHOPIFY_IP_WHITELIST",
        "TPG_USE_X_FORWARDED_FOR",
        "TPG_USE_FORWARDED",
        "TPG_UNCLAIMED_ORDER_TIMEOUT",
        "TPG_UNPAID_ORDER_TIMEOUT",
        "TPG_SKIP_PREFLIGHT",
        "TPG_PAYMENT_WALLET_ADDRESS",
    ];

    println!("Current environment values (EXCLUDING variables that contain secrets):");
    DISPLAY_ENVS.iter().for_each(|&name| {
        let val = match env::var(name) {
            Ok(s) => s,
            Err(VarError::NotPresent) => "Not set".into(),
            Err(VarError::NotUnicode(s)) => format!("Invalid value: {}", s.to_string_lossy()),
        };
        println!("  {name:<35} {val:<15}");
    })
}
