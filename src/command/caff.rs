use orphism::caff::Archive;
use std::{
  fs::File,
  io::{Cursor, Write},
  path::PathBuf,
};

#[derive(Debug, Clone, clap::Parser)]
#[remain::sorted]
#[clap(about = "tools for working with CAFF archives (such as .cmo3 and .can3 files)")]
pub struct Caff {
  #[arg(long, help = "path to a valid CAFF archive")]
  archive: PathBuf,
  #[command(subcommand)]
  subcommand: Subcommand,
}

#[derive(Debug, Clone, clap::Subcommand)]
#[remain::sorted]
enum Subcommand {
  // Decrypt(Decrypt),
  Extract(Extract),
  List(List),
  ShowKey(ShowKey),
}

impl Caff {
  pub fn execute(self) -> anyhow::Result<()> {
    let Self { archive, subcommand } = self;

    let mut archive = File::open(&archive)?;
    let mut archive = Archive::read(&mut archive)?;

    match subcommand {
      // Subcommand::Decrypt(command) => command.execute(&mut archive),
      Subcommand::Extract(command) => command.execute(archive),
      Subcommand::List(command) => command.execute(&mut archive),
      Subcommand::ShowKey(command) => command.execute(&mut archive),
    }
  }
}

#[derive(Debug, Clone, clap::Parser)]
#[remain::sorted]
#[clap(about = "show the encryption key of a CAFF archive")]
struct ShowKey {
  #[arg(long, short, default_value = "dec", help = "format to print the key in")]
  format: KeyFormat,
  #[arg(long = "without-prefix", action = clap::ArgAction::SetFalse, help = "prefix key with 0x (when hex) or 0b (when bin)")]
  prefix: bool,
}

impl ShowKey {
  fn execute(&self, archive: &mut Archive) -> anyhow::Result<()> {
    let Self { format, prefix } = self;
    let key = u32::from(archive.header.key);

    match (format, prefix) {
      (KeyFormat::Bin, false) => println!("{key:034b}"),
      (KeyFormat::Bin, true) => println!("{key:#034b}"),
      (KeyFormat::Dec, _) => println!("{key}"),
      (KeyFormat::Hex, false) => println!("{key:010X?}"),
      (KeyFormat::Hex, true) => println!("{key:#010X?}"),
    }

    Ok(())
  }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
#[remain::sorted]
enum KeyFormat {
  Bin,
  Dec,
  Hex,
}

#[derive(Debug, Clone, clap::Parser)]
#[remain::sorted]
#[clap(about = "list the contents of a CAFF archive")]
struct List {
  #[arg(long = "no-header", short = 'H', action = clap::ArgAction::SetFalse, help = "skip printing column header")]
  header: bool,
  #[arg(long, short, help = "include file sizes in output")]
  sizes: bool,
  #[arg(long, short, help = "include tags in output")]
  tags: bool,
}

impl List {
  fn execute(&self, archive: &mut Archive) -> anyhow::Result<()> {
    let Self { header, sizes, tags } = self;

    if *header {
      let header = match (sizes, tags) {
        (false, false) => "FILENAME",
        (true, false) => "FILENAME\tSIZE",
        (false, true) => "FILENAME\tTAG",
        (true, true) => "FILENAME\tSIZE\tTAG",
      };
      println!("{header}");
    }

    for metadata in archive.body.metadata.iter() {
      let entry = match (sizes, tags) {
        (false, false) => format!("{}", metadata.file_name),
        (true, false) => format!("{}\t{}", metadata.file_name, metadata.file_size),
        (false, true) if metadata.tag.is_empty() => format!("{}", metadata.file_name),
        (false, true) => format!("{}\t{}", metadata.file_name, metadata.tag),
        (true, true) if metadata.tag.is_empty() => format!("{}\t{}", metadata.file_name, metadata.file_size),
        (true, true) => format!("{}\t{}\t{}", metadata.file_name, metadata.file_size, metadata.tag),
      };
      println!("{entry}");
    }

    Ok(())
  }
}

// #[derive(Debug, Clone, clap::Parser)]
// #[remain::sorted]
// struct Decrypt {
//   #[arg(long, short, value_name = "FILE")]
//   output: PathBuf,
//   #[arg(long, short, value_name = "KEY")]
//   recrypt: Option<u32>,
// }

// impl Decrypt {
//   pub fn execute(&self, archive: &mut Archive) -> anyhow::Result<()> {
//     let Self { output, recrypt } = self;
//     let key = recrypt.map(Key::from).unwrap_or_default();
//     archive.header.key = key;
//     if let Some(parent) = output.parent() {
//       if !parent.exists() {
//         std::fs::create_dir_all(parent)?;
//       }
//     }
//     let mut file = File::create(output)?;
//     archive.write(&mut file)?;
//     file.flush()?;
//     Ok(())
//   }
// }

#[derive(Debug, Clone, clap::Parser)]
#[clap(about = "extract files from a CAFF archive")]
struct Extract {
  #[arg(value_name = "ENTRY", help = "a list of filenames to extract from the archive")]
  entries: Vec<String>,
  #[arg(long, short, value_name = "DIR", default_value = "output", help = "a directory to extract into")]
  output: PathBuf,
  #[arg(long, help = "entries refer to tags rather than filenames")]
  tagged: bool,
  #[arg(long, short, help = "verbose output")]
  verbose: bool,
  #[arg(
    long,
    value_name = "FEATURE",
    default_value = "unpack",
    help = "controls the amount of automagical changes during extraction",
    long_help = "controls the amount of automagical changes during extraction: [none] does nothing, [fix] adds missing ZIP Central Directory sections, [rename] does that AND adds a .zip suffix (if missing), [rewrite] does that AND renames the archive contents to the original name of the file, and [unpack] decompresses the content in-place instead of doing any of that."
  )]
  zip_automagic: ZipAutomagic,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
enum ZipAutomagic {
  None,
  Fix,
  Rename,
  Rewrite,
  #[default]
  Unpack,
}

impl Extract {
  pub fn execute(&self, archive: Archive) -> anyhow::Result<()> {
    let Self {
      entries,
      output,
      tagged,
      verbose,
      zip_automagic,
    } = self;

    if !output.exists() {
      std::fs::create_dir_all(&output)?;
    }

    for (metadata, data) in archive.body.metadata.into_iter().zip(archive.body.data) {
      let qualifying_tag = *tagged && !metadata.tag.is_empty() && (entries.is_empty() || entries.contains(&metadata.tag));
      let qualifying_file = !*tagged && (entries.is_empty() || entries.contains(&metadata.file_name));
      if qualifying_tag || qualifying_file {
        if metadata.tag == "main_xml" {
          let path = match zip_automagic {
            ZipAutomagic::None | ZipAutomagic::Fix | ZipAutomagic::Unpack => output.join(&metadata.file_name),
            ZipAutomagic::Rename | ZipAutomagic::Rewrite if metadata.file_name.ends_with(".zip") => output.join(&metadata.file_name),
            ZipAutomagic::Rename | ZipAutomagic::Rewrite => output.join(metadata.file_name.clone() + ".zip"),
          };

          if *verbose {
            println!("extract: {} ({} bytes)", &metadata.file_name, &metadata.file_size);
          }

          let mut file = File::create(&path)?;

          if *zip_automagic == ZipAutomagic::None {
            file.write_all(&data)?;
          } else {
            let mut reader = Cursor::new(data);
            let mut entry = synthzip::Entry::read(&mut reader)?;

            if *zip_automagic == ZipAutomagic::Unpack {
              let data = entry.decompress()?;
              file.write_all(&data)?;
            } else {
              if *zip_automagic == ZipAutomagic::Rewrite {
                entry.header.file_name = metadata.file_name;
              }
              let mut cd = synthzip::CentralDirectory::new();
              cd.add(&entry)?;
              entry.write(&mut file)?;
              cd.write(&mut file)?;
            }
          }

          file.flush()?;
        } else {
          let path = output.join(&metadata.file_name);
          if *verbose {
            println!("extract: {} ({} bytes)", &metadata.file_name, &metadata.file_size);
          }

          let mut file = File::create(&path)?;
          file.write_all(&data)?;
          file.flush()?;
        }
      }
    }

    Ok(())
  }
}
