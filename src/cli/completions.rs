use {
    super::args::{Cli, Shell},
    clap::CommandFactory,
    clap_complete::generate,
    std::io,
};

pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}
