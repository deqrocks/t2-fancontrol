use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{FanControlError, Result};

#[derive(Clone, Debug)]
pub struct FanEndpoint {
    pub name: String,
    pub base_path: PathBuf,
    pub min_speed: u32,
    pub max_speed: u32,
    pub current_speed: Option<u32>,
    pub manual_enabled: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct TemperatureSource {
    pub name: String,
    pub path: PathBuf,
    pub last_temp_c: Option<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct TemperatureSnapshot {
    pub cpu_temp_c: Option<u8>,
    pub gpu_temp_c: Option<u8>,
}

impl TemperatureSnapshot {
    pub fn read_from(sources: &mut [TemperatureSource]) -> Self {
        let mut snapshot = Self::default();
        for source in sources {
            source.last_temp_c = read_temperature(&source.path).ok();
            match source.name.as_str() {
                "CPU" => snapshot.cpu_temp_c = source.last_temp_c,
                "GPU" => snapshot.gpu_temp_c = source.last_temp_c,
                _ => {}
            }
        }
        snapshot
    }

    pub fn effective_temp_c(&self) -> Option<u8> {
        match (self.cpu_temp_c, self.gpu_temp_c) {
            (Some(cpu), Some(gpu)) => Some(cpu.max(gpu)),
            (Some(cpu), None) => Some(cpu),
            (None, Some(gpu)) => Some(gpu),
            (None, None) => None,
        }
    }
}

impl FanEndpoint {
    pub fn refresh_state(&mut self) -> Result<()> {
        self.current_speed = Some(read_u32(&join_suffix(&self.base_path, "_input"))?);
        self.manual_enabled = Some(read_u32(&join_suffix(&self.base_path, "_manual"))? != 0);
        Ok(())
    }

    pub fn set_manual(&self, enabled: bool) -> Result<()> {
        write_string(
            &join_suffix(&self.base_path, "_manual"),
            if enabled { "1" } else { "0" },
        )
    }

    pub fn set_speed(&self, requested_speed: u32) -> Result<()> {
        let clamped = requested_speed.clamp(self.min_speed, self.max_speed);
        write_string(&join_suffix(&self.base_path, "_output"), &clamped.to_string())
    }

    pub fn percent_to_rpm(&self, percent: u8) -> u32 {
        let span = self.max_speed.saturating_sub(self.min_speed);
        self.min_speed + (span * percent as u32 / 100)
    }
}

pub fn discover_fans() -> Result<Vec<FanEndpoint>> {
    let first_fan = glob::glob("/sys/devices/pci*/*/*/*/APP0001:00/fan*_input")?
        .filter_map(std::result::Result::ok)
        .next()
        .ok_or(FanControlError::NoFans)?;

    let fan_dir = first_fan
        .parent()
        .ok_or_else(|| FanControlError::InvalidFanPath(first_fan.clone()))?;
    let pattern = format!("{}/fan*_input", fan_dir.display());

    let mut fans = Vec::new();
    for entry in glob::glob(&pattern)? {
        let input_path = entry?;
        let fan_path = input_to_base_path(&input_path)?;
        let name = fan_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| FanControlError::InvalidFanPath(fan_path.clone()))?
            .to_owned();

        let min_speed = read_u32(&join_suffix(&fan_path, "_min"))?;
        let max_speed = read_u32(&join_suffix(&fan_path, "_max"))?;
        let current_speed = read_u32(&input_path).ok();
        let manual_enabled = read_u32(&join_suffix(&fan_path, "_manual")).ok().map(|value| value != 0);

        fans.push(FanEndpoint {
            name,
            base_path: fan_path,
            min_speed,
            max_speed,
            current_speed,
            manual_enabled,
        });
    }

    fans.sort_by(|left, right| left.name.cmp(&right.name));
    if fans.is_empty() {
        return Err(FanControlError::NoFans);
    }
    Ok(fans)
}

pub fn discover_temperature_sources() -> Vec<TemperatureSource> {
    let mut sources = Vec::new();

    if let Some(path) = first_existing_path("/sys/devices/platform/coretemp.0/hwmon/hwmon*/temp1_input") {
        sources.push(TemperatureSource {
            name: String::from("CPU"),
            path,
            last_temp_c: None,
        });
    }

    if let Some(path) = first_existing_path("/sys/class/drm/card0/device/hwmon/hwmon*/temp*_input") {
        sources.push(TemperatureSource {
            name: String::from("GPU"),
            path,
            last_temp_c: None,
        });
    }

    sources
}

fn first_existing_path(pattern: &str) -> Option<PathBuf> {
    let paths = glob::glob(pattern).ok()?;
    for entry in paths {
        let Ok(path) = entry else {
            continue;
        };
        if path.exists() && read_temperature(&path).is_ok() {
            return Some(path);
        }
    }
    None
}

fn input_to_base_path(input_path: &Path) -> Result<PathBuf> {
    let file_name = input_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| FanControlError::InvalidFanPath(input_path.to_path_buf()))?;
    let fan_name = file_name
        .strip_suffix("_input")
        .ok_or_else(|| FanControlError::InvalidFanPath(input_path.to_path_buf()))?;

    Ok(input_path.with_file_name(fan_name))
}

fn join_suffix(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("fan"));
    path.with_file_name(format!("{file_name}{suffix}"))
}

fn read_u32(path: &Path) -> Result<u32> {
    let contents = fs::read_to_string(path).map_err(|source| FanControlError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    contents
        .trim()
        .parse::<u32>()
        .map_err(|source| FanControlError::ParseInt {
            path: path.to_path_buf(),
            source,
        })
}

fn read_temperature(path: &Path) -> Result<u8> {
    let raw = read_u32(path)?;
    Ok((raw / 1000) as u8)
}

fn write_string(path: &Path, value: &str) -> Result<()> {
    fs::write(path, value).map_err(|source| FanControlError::Io {
        path: path.to_path_buf(),
        source,
    })
}
