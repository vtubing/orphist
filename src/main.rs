use clap::Parser;
use simple_logger::SimpleLogger;

mod command;

use command::Command;

fn main() -> anyhow::Result<()> {
  let command = Command::parse();

  SimpleLogger::from(command.log_level).env().init()?;

  command.execute()
}
