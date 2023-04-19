use clap_complete::{generate_to, shells};
use std::env;
use std::io::Error;

include!("cli/src/args.rs");

fn main() -> Result<(), Error> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    let mut cmd = Arguments::command();
    let bash_path = generate_to(
        shells::Bash,
        &mut cmd,       // We need to specify what generator to use
        "void-cli",     // We need to specify the bin name manually
        outdir.clone(), // We need to specify where to write to
    )?;

    let fish_path = generate_to(
        shells::Fish,
        &mut cmd,       // We need to specify what generator to use
        "void-cli",     // We need to specify the bin name manually
        outdir.clone(), // We need to specify where to write to
    )?;

    let zsh_path = generate_to(
        shells::Zsh,
        &mut cmd,       // We need to specify what generator to use
        "void-cli",     // We need to specify the bin name manually
        outdir.clone(), // We need to specify where to write to
    )?;

    let ps_path = generate_to(
        shells::PowerShell,
        &mut cmd,       // We need to specify what generator to use
        "void-cli",     // We need to specify the bin name manually
        outdir.clone(), // We need to specify where to write to
    )?;

    let elvish_path = generate_to(
        shells::Elvish,
        &mut cmd,       // We need to specify what generator to use
        "void-cli",     // We need to specify the bin name manually
        outdir.clone(), // We need to specify where to write to
    )?;

    println!("cargo:warning=bash completion files are generated: {bash_path:?}");
    println!("cargo:warning=fish completion files are generated: {fish_path:?}");
    println!("cargo:warning=zsh completion files are generated: {zsh_path:?}");
    println!("cargo:warning=power shell completion files are generated: {ps_path:?}");
    println!("cargo:warning=elvish completion files are generated: {elvish_path:?}");

    let man = clap_mangen::Man::new(cmd);
    let man_path = std::path::PathBuf::from(outdir).join("void-cli.1");
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    std::fs::write(man_path.clone(), buffer)?;

    println!("cargo:warning=man page generated: {man_path:?}");

    Ok(())
}
