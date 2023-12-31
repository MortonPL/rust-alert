// CSF to INI and INI to CSF tool.
use std::{
    fs::OpenOptions,
    io::BufReader,
    path::PathBuf,
};

use clap::{Parser, Subcommand};

use rust_alert::core::{
    csf::{CsfLanguageEnum, CsfStringtable, CsfVersionEnum},
    csf_io::{CsfReader, CsfWriter},
    ini::IniFile,
    ini_io::{IniReader, IniWriter},
};

#[derive(Parser)]
#[command(name = "stringer")]
#[command(author = "MortonPL <bartm12@wp.pl>")]
#[command(version = "1.0")]
#[command(about = "Extract CSF to INI and vice versa.", long_about = None)]
struct Args {
    #[command(subcommand)]
    /// Mode of operation.
    command: Commands,
}

#[derive(clap::Args)]
struct ExtractArgs {
    /// Path to an input CSF file.
    input: PathBuf,
    /// Path to an output INI file.
    output: PathBuf,
    /// Sort all strings.
    #[arg(short, long, default_value_t = false)]
    sort: bool,
}

#[derive(clap::Args)]
struct BuildArgs {
    /// Path to an input INI file.
    input: PathBuf,
    /// Path to an output CSF file.
    output: PathBuf,
    /// CSF language ID.
    #[arg(short, long, value_enum, default_value_t = CsfLanguageEnum::ENUS)]
    language: CsfLanguageEnum,
    /// CSF format version.
    #[arg(short, long, value_enum, default_value_t = CsfVersionEnum::Cnc)]
    version: CsfVersionEnum,
    /// Sort all strings.
    #[arg(short, long, default_value_t = false)]
    sort: bool,
}

/// Modes of operation.
#[derive(Subcommand)]
enum Commands {
    /// Extract stringtable contents to INI.
    Extract(ExtractArgs),
    /// Build stringtable from INI.
    Build(BuildArgs),
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    CsfError(#[from] rust_alert::core::csf::Error),
    #[error("{0}")]
    IniError(#[from] rust_alert::core::ini_io::Error),
    #[error("Label {0} contains no strings")]
    EmptyLabel(String),
    #[error("Label {0} is not in CATEGORY:NAME format, which is required")]
    NoSplit(String),
}

type Result<T> = std::result::Result<T, Error>;

fn extract_csf_to_ini(csf: CsfStringtable) -> Result<IniFile> {
    let mut ini = IniFile::default();
    for (name, label) in csf.iter() {
        let value = &label
            .get_first()
            .ok_or(Error::EmptyLabel(name.to_string()))?
            .value
            .replace('\n', "\\n");
        let mut iter = name.split(':');
        let kv = match (iter.next(), iter.next()) {
            (Some(k), Some(v)) => Ok((k, v)),
            _ => Err(Error::NoSplit(name.to_string())),
        }?;
        ini.add_to_section(kv.0, kv.1, value);
    }
    Ok(ini)
}

fn build_csf_from_ini(ini: IniFile) -> Result<CsfStringtable> {
    let mut csf = CsfStringtable::default();
    for (name, section) in ini.iter() {
        for (key, value) in section.iter() {
            let value = &value.value.replace("\\n", "\n");
            csf.create_label(format!("{name}:{key}"), value);
        }
    }
    Ok(csf)
}

fn extract(args: &ExtractArgs) -> Result<()> {
    let mut reader = OpenOptions::new().read(true).open(args.input.clone())?;
    let mut writer = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(args.output.clone())?;
    let csf = CsfReader::read_file(&mut reader)?;
    let mut ini = extract_csf_to_ini(csf)?;
    if args.sort {
        ini.sort_all();
    }
    IniWriter::write_file(&ini, &mut writer)?;
    Ok(())
}

fn build(args: &BuildArgs) -> Result<()> {
    let reader = OpenOptions::new().read(true).open(args.input.clone())?;
    let mut reader = BufReader::new(reader);
    let mut writer = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(args.output.clone())?;
    let mut ini = IniReader::read_file(&mut reader)?;
    if args.sort {
        ini.sort_all()
    }
    let mut csf = build_csf_from_ini(ini)?;
    csf.language = args.language;
    csf.version = args.version;
    CsfWriter::write_file(&csf, &mut writer)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse_from(wild::args());

    match &args.command {
        Commands::Extract(e) => extract(e),
        Commands::Build(b) => build(b),
    }
}
