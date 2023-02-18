use crate::parser::{
    parse_meminfo, parse_nix_store_path, parse_stat, parse_swaps, parse_uptime, MemInfo, Stat,
    Swaps,
};
use serde::Serialize;
use std::{fs, ops::Sub, str, time::Duration};

#[derive(Debug)]
pub enum MetricError {
    FileRead(String),
    LinkRead(String),
    MetricParse(String),
}

fn read_file(fp: &str) -> Result<String, MetricError> {
    let s = fs::read_to_string(fp);
    match s {
        Ok(s) => Ok(s),
        Err(_) => Err(MetricError::FileRead(format!("Unable to read {}", fp))),
    }
}

fn read_link(fp: &str) -> Result<String, MetricError> {
    let link = fs::read_link(fp)
        .ok()
        .and_then(|l| l.to_str().map(String::from));
    match link {
        Some(l) => Ok(l),
        None => Err(MetricError::LinkRead(format!("Unable to read link {}", fp))),
    }
}

fn read_nixos_current_system() -> Result<String, MetricError> {
    let link = read_link("/run/current-system")?;
    let parsed_link = parse_nix_store_path(&link);
    match parsed_link {
        Ok((_, current_system)) => Ok(current_system.to_string()),
        Err(_) => Err(MetricError::MetricParse(
            "Unable to parse current system".to_string(),
        )),
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Cpu {
    pub total: usize,
    pub used: usize,
}

impl From<Stat> for Cpu {
    fn from(s: Stat) -> Cpu {
        let total = s.user
            + s.nice
            + s.system
            + s.idle
            + s.iowait
            + s.irq
            + s.softirq
            + s.steal
            + s.guest
            + s.guest_nice;
        Cpu {
            total,
            used: total - s.idle,
        }
    }
}

impl<'a, 'b> Sub<&'b Cpu> for &'a Cpu {
    type Output = Cpu;

    fn sub(self, other: &'b Cpu) -> Cpu {
        Cpu {
            total: self.total - other.total,
            used: self.used - other.used,
        }
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Memory {
    pub total: usize,
    pub used: usize,
}

impl From<MemInfo> for Memory {
    fn from(m: MemInfo) -> Memory {
        let total = m.get("MemTotal").expect("Expected MemTotal in meminfo");
        let available = m
            .get("MemAvailable")
            .expect("Expected MemAvailable in meminfo");
        Memory {
            total: *total,
            used: total - available,
        }
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Swap {
    pub size: usize,
    pub used: usize,
}

impl From<Swaps> for Swap {
    fn from(s: Swaps) -> Swap {
        let (size, used) = s
            .into_values()
            .map(|x| (x.size, x.used))
            .fold((0, 0), |(acc_s, acc_u), (size, used)| {
                (acc_s + size, acc_u + used)
            });
        Swap { size, used }
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Metrics {
    pub uptime: Duration,
    pub cpu_since_boot: Cpu,
    pub cpu_delta: Cpu,
    pub memory: Memory,
    pub swap: Swap,
    pub current_system: Option<String>,
}

fn get_metric<T>(fp: &str, f: fn(&str) -> nom::IResult<&str, T>) -> Result<T, MetricError> {
    let metric = read_file(fp)?;
    match f(&metric) {
        Ok((_, parsed_metric)) => Ok(parsed_metric),
        Err(_) => Err(MetricError::MetricParse(format!(
            "Unable to parse metric from {}",
            fp
        ))),
    }
}

pub fn get_metrics(
    last_metrics: &Metrics,
    get_current_system: bool,
) -> Result<Metrics, MetricError> {
    let memory = Memory::from(get_metric("/proc/meminfo", parse_meminfo)?);
    let uptime = get_metric("/proc/uptime", parse_uptime)?;
    let swap = Swap::from(get_metric("/proc/swaps", parse_swaps)?);
    let cpu_since_boot = Cpu::from(get_metric("/proc/stat", parse_stat)?);
    let cpu_delta = &cpu_since_boot - &last_metrics.cpu_since_boot;
    let current_system = if get_current_system {
        Some(read_nixos_current_system()?)
    } else {
        None
    };

    Ok(Metrics {
        uptime,
        cpu_since_boot,
        cpu_delta,
        memory,
        swap,
        current_system,
    })
}
