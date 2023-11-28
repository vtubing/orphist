mod analyze;
mod caff;
mod load;

#[derive(Debug, Clone, clap::Parser)]
#[remain::sorted]
pub struct Command {
  #[arg(long, value_name = "LEVEL", default_value = "info")]
  pub log_level: LogLevel,
  #[command(subcommand)]
  subcommand: Subcommand,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Subcommand {
  Analyze(analyze::Analyze),
  Caff(caff::Caff),
  Load(load::Load),
}

impl Command {
  pub fn execute(self) -> anyhow::Result<()> {
    let Self { subcommand, .. } = self;

    match subcommand {
      Subcommand::Analyze(command) => command.execute(),
      Subcommand::Caff(command) => command.execute(),
      Subcommand::Load(command) => command.execute(),
    }
  }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
#[remain::sorted]
pub enum LogLevel {
  Debug,
  Error,
  Info,
  Trace,
  Warn,
}

impl From<LogLevel> for log::Level {
  #[remain::check]
  fn from(level: LogLevel) -> Self {
    #[remain::sorted]
    match level {
      LogLevel::Debug => Self::Debug,
      LogLevel::Error => Self::Error,
      LogLevel::Info => Self::Info,
      LogLevel::Trace => Self::Trace,
      LogLevel::Warn => Self::Warn,
    }
  }
}

impl From<LogLevel> for log::LevelFilter {
  fn from(level: LogLevel) -> Self {
    log::Level::from(level).to_level_filter()
  }
}

impl From<LogLevel> for simple_logger::SimpleLogger {
  fn from(level: LogLevel) -> Self {
    let level = level.into();
    Self::new().with_level(level)
  }
}
