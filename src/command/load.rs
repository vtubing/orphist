use orphism::{Runtime, RuntimeError};
use std::io::Read;

#[derive(Debug, Clone, clap::Parser)]
#[remain::sorted]
pub struct Load {
  #[arg(long, value_name = "FILENAME")]
  match_filename: Option<String>,
  #[arg(long)]
  moc3: bool,
  #[arg(long, value_name = "GLOB", default_value = "./assets/**/*.model3.json")]
  pattern: String,
}

impl Load {
  pub fn execute(self) -> anyhow::Result<()> {
    let Self {
      match_filename: only_filename,
      moc3,
      pattern,
    } = self;

    let mut models = Vec::new();

    log::info!("looking for files matching {pattern:?}");

    for model in glob::glob(&pattern)? {
      let model = model?;

      if let (Some(file_name), Some(target)) = (model.file_name(), only_filename.as_ref()) {
        if !file_name.to_string_lossy().eq(target) {
          log::debug!("skipping {file_name:?} because it does not match {target:?}");
          continue;
        }
      }

      log::debug!("found {model:?}");

      if let Some(root) = model.as_path().parent() {
        log::debug!("attempting to load directory {root:?}");
        let runtime = match Runtime::new_from_runtime_path(root.to_owned()) {
          Ok(runtime) => {
            log::info!("loaded model from directory {root:?}");
            runtime
          }
          Err(RuntimeError::RuntimePathContainsMultipleModels(_)) => {
            log::debug!("failed because directory contains multiple models, attempting to load single model");
            match Runtime::new_from_model_path(model.clone()) {
              Ok(runtime) => {
                log::info!("loaded model from file: {model:?}");
                runtime
              }
              Err(error) => Err(error)?,
            }
          }
          Err(error) => Err(error)?,
        };
        let model = runtime.load()?;
        let mut header = [0u8; 64];
        model.data.moc.clone().take(64).read_exact(&mut header)?;
        models.push(model);
      }
    }

    log::info!("successfully loaded {} models", models.len());

    if moc3 {
      log::info!("attempting to parse .moc3 data from all loaded models");

      for model in models {
        let model = orphism::moc3::Model::read(model.data.moc)?;

        println!("{model:#?}");
      }
    }

    Ok(())
  }
}
