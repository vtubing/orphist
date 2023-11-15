use log::{debug, info, trace};
use orphism::Runtime;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Parser)]
#[remain::sorted]
pub struct Analyze {
  #[arg(long, default_value = "little")]
  endian: Endian,
  #[arg(long, conflicts_with = "runtime_dir")]
  model_file: Option<PathBuf>,
  #[arg(long, default_value = "5")]
  report_offset: u64,
  #[arg(long, conflicts_with = "model_file")]
  runtime_dir: Option<PathBuf>,
  #[arg(long, default_value = "0")]
  start_at: u64,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, strum::EnumString, strum::Display)]
#[strum(serialize_all = "lowercase")]
enum Endian {
  Big,
  Little,
}

impl Analyze {
  pub fn execute(self) -> anyhow::Result<()> {
    let Self {
      endian,
      model_file: model,
      report_offset,
      runtime_dir: runtime,
      start_at,
    } = self;

    let runtime = match (model, runtime) {
      (Some(_), Some(_)) => panic!("Cannot provide both model and runtime path. CLI argument validation should have prevented this. (╯°□°)╯︵ ┻━┻"),
      (None, None) => panic!("Missing either model or runtime path."),
      (Some(path), None) => Runtime::new_from_model_path(path)?,
      (None, Some(path)) => Runtime::new_from_runtime_path(path)?,
    };

    let model = runtime.load()?;

    let mut moc3 = Cursor::new(model.data.moc);
    moc3.seek(SeekFrom::Start(start_at))?;
    let mut buf = [0u8; 4];

    let mut data_run = 0u64;
    let mut data_start = moc3.stream_position()?;
    let mut zero_run = 0u64;
    let mut zero_start = moc3.stream_position()?;

    let mut data = Vec::<[u8; 4]>::new();
    let mut last = moc3.stream_position()?;

    while let Ok(()) = moc3.read_exact(&mut buf) {
      if buf == [0, 0, 0, 0] {
        if zero_run == 0 {
          zero_start = last;
        } else {
          if data_run > 0 {
            let (assumed, min, max, maybe_float, maybe_string) = infer(&data, endian);
            info!(
              "DATA {:#010x?} {:#010x?} size={} probably={assumed} min={min} max={max} maybe_float={maybe_float} maybe_string={maybe_string}",
              data_start,
              last - report_offset,
              data_run * 4
            );
          }
          data_run = 0;
        }
        zero_run += 1;
      } else {
        if data_run == 0 {
          data_start = last;
          data.clear();
        } else {
          if zero_run >= 8 {
            debug!("VOID {:#010x?} {:#010x?} size={}", zero_start, last - report_offset, zero_run * 4);
          }
          zero_run = 0;
        }
        data.push(buf);
        data_run += 1
      }

      last = moc3.stream_position()?;
    }

    Ok(())
  }
}

fn infer(data: &[[u8; 4]], endian: Endian) -> (AssumedType, i64, i64, bool, bool) {
  let mut all = Vec::new();
  let mut min = 0i64;
  let mut max = 0i64;
  let mut min_f = 0.0f32;
  let mut max_f = 0.0f32;
  let mut float = true;

  for bytes in data {
    {
      let number = match endian {
        Endian::Big => i32::from_be_bytes(*bytes),
        Endian::Little => i32::from_le_bytes(*bytes),
      }
      .into();

      trace!("{number} as {endian} signed");

      if number < min {
        min = number;
      }

      if number > max {
        max = number;
      }
    }

    {
      let number = match endian {
        Endian::Big => u32::from_be_bytes(*bytes),
        Endian::Little => u32::from_le_bytes(*bytes),
      }
      .into();

      trace!("{number} as {endian} unsigned");

      if number < min {
        min = number;
      }

      if number > max {
        max = number;
      }
    }

    {
      let number = match endian {
        Endian::Big => f32::from_be_bytes(*bytes),
        Endian::Little => f32::from_le_bytes(*bytes),
      };

      trace!("{number} as {endian} float");

      if number < min_f {
        min_f = number;
      }

      if number > max_f {
        max_f = number;
      }

      if number.is_nan() {
        float = false;
      }
    }

    for byte in bytes {
      all.push(*byte);
    }
  }

  let string = String::from_utf8(all).is_ok();

  let assumed_type = if (min == 0 || min == 1) && (max == 0 || max == 1) {
    AssumedType::Bool
  } else if min.is_negative() || max.is_negative() {
    if (min >= i8::MIN.into()) && (max <= i8::MAX.into()) {
      AssumedType::I8
    } else if (min >= i16::MIN.into()) && (max <= i16::MAX.into()) {
      AssumedType::I16
    } else {
      AssumedType::I32
    }
  } else if max.is_positive() {
    if (min >= u8::MIN.into()) && (max <= u8::MAX.into()) {
      AssumedType::U8
    } else if (min >= u16::MIN.into()) && (max <= u16::MAX.into()) {
      AssumedType::U16
    } else {
      AssumedType::U32
    }
  } else {
    AssumedType::Zero
  };

  (assumed_type, min, max, float, string)
}

#[derive(Debug, Clone, Copy, strum::Display)]
#[allow(non_camel_case_types)]
#[remain::sorted]
enum AssumedType {
  Bool,
  I8,
  I16,
  I32,
  U8,
  U16,
  U32,
  Zero,
}
