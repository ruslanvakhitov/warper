use clap::{CommandFactory, Parser};

use super::{Args, Command};

#[test]
fn built_in_ai_commands_are_not_registered() {
    for command in ["agent", "mcp", "model", "provider", "secret"] {
        assert!(
            Args::try_parse_from(["warper", command]).is_err(),
            "removed command accepted: {command}"
        );
    }
}

#[test]
fn provider_flags_are_not_registered() {
    for option in ["--api-key", "--output-format"] {
        assert!(Args::try_parse_from(["warper", option, "value"]).is_err());
    }
}

#[test]
fn terminal_commands_remain_available() {
    let args = Args::try_parse_from(["warper", "completions", "zsh"]).unwrap();
    assert!(matches!(args.command(), Some(Command::Completions { .. })));
    let args = Args::try_parse_from(["warper", "--dump-debug-info"]).unwrap();
    assert!(matches!(args.command(), Some(Command::DumpDebugInfo)));
    assert!(
        Args::try_parse_from(["warper"])
            .unwrap()
            .command()
            .is_none()
    );
}

#[test]
fn help_describes_terminal_commands() {
    let help = <Args as CommandFactory>::command()
        .render_long_help()
        .to_string();
    assert!(help.contains("Warper terminal"));
    assert!(help.contains("completions"));
    for removed in ["agent run", "mcp", "provider", "api-key", "output-format"] {
        assert!(
            !help.contains(removed),
            "help contains removed feature: {removed}"
        );
    }
}
