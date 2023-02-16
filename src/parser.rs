use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until},
    character::complete::{alphanumeric1, char, digit1, line_ending, multispace0},
    combinator::{map, map_res},
    error::ParseError,
    multi::many0,
    number::complete::double,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use std::collections::HashMap;
use std::str;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub struct Stat {
    user: usize,
    nice: usize,
    system: usize,
    idle: usize,
    iowait: usize,
    irq: usize,
    softirq: usize,
    steal: usize,
    guest: usize,
    guest_nice: usize,
}

impl Stat {
    pub fn average_idle(&self) -> usize {
        (self.idle * 100)
            / (self.user
                + self.nice
                + self.system
                + self.idle
                + self.iowait
                + self.irq
                + self.softirq
                + self.steal
                + self.guest
                + self.guest_nice)
    }
}

#[derive(Debug, PartialEq)]
pub struct MemInfo {
    total: usize,
    available: usize,
}

impl MemInfo {
    pub fn total_used(&self) -> usize {
        ((self.total - self.available) * 100) / self.total
    }
}

#[derive(Debug, PartialEq)]
pub struct Swap {
    size: usize,
    used: usize,
}

impl Swap {
    pub fn total_used(&self) -> usize {
        (self.used * 100) / self.size
    }
}

fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_usize(i: &str) -> IResult<&str, usize> {
    map_res(ws(digit1), str::FromStr::from_str)(i)
}

fn parse_isize(i: &str) -> IResult<&str, isize> {
    map_res(
        ws(alt((preceded(char('-'), digit1), digit1))),
        str::FromStr::from_str,
    )(i)
}

fn parse_f64(i: &str) -> IResult<&str, f64> {
    ws(double)(i)
}

pub fn parse_stat(i: &str) -> IResult<&str, Stat> {
    let (i, _) = take_until("cpu ")(i)?;
    let parser = tuple((
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
        parse_usize,
    ));

    let (i, (user, nice, system, idle, iowait, irq, softirq, steal, guest, guest_nice)) =
        preceded(tag("cpu "), parser)(i)?;
    Ok((
        i,
        Stat {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            guest,
            guest_nice,
        },
    ))
}

pub fn parse_uptime(i: &str) -> IResult<&str, Duration> {
    let (i, u) = parse_f64(i)?;
    Ok((i, Duration::from_secs_f64(u)))
}

pub fn parse_meminfo(i: &str) -> IResult<&str, MemInfo> {
    let (i, total) = delimited(tag("MemTotal:"), parse_usize, pair(tag("kB"), line_ending))(i)?;
    let (i, _) = delimited(tag("MemFree:"), parse_usize, pair(tag("kB"), line_ending))(i)?;
    let (i, available) = delimited(
        tag("MemAvailable:"),
        parse_usize,
        pair(tag("kB"), line_ending),
    )(i)?;
    Ok((i, MemInfo { total, available }))
}

pub fn parse_swaps(i: &str) -> IResult<&str, HashMap<String, Swap>> {
    let (i, _) = tuple((
        ws(tag("Filename")),
        ws(tag("Type")),
        ws(tag("Size")),
        ws(tag("Used")),
        ws(pair(tag("Priority"), line_ending)),
    ))(i)?;

    map(many0(parse_swap_line), |swaps| swaps.into_iter().collect())(i)
}

pub fn parse_swap_line(i: &str) -> IResult<&str, (String, Swap)> {
    let (i, filename) = take_till(char::is_whitespace)(i)?;
    let (i, (_type, size, used, _priority)) =
        tuple((ws(alphanumeric1), parse_usize, parse_usize, parse_isize))(i)?;
    Ok((i, (filename.to_string(), Swap { size, used })))
}

#[test]
fn parse_proc_stat() {
    let proc_stat = "cpu  9701702 6293 1291945 119400172 120770 0 120369 0 0 0
cpu0 1209513 784 169115 14910230 15511 0 34945 0 0 0
cpu1 1209721 776 161430 14923348 15558 0 15489 0 0 0
cpu2 1217037 764 158973 14942082 15003 0 13775 0 0 0
cpu3 1307743 793 163042 14833254 14664 0 4384 0 0 0
cpu4 1205766 755 153402 14966185 14950 0 8169 0 0 0
cpu5 1215377 766 152806 14948197 15296 0 13306 0 0 0
cpu6 1218276 832 158639 14917222 14966 0 4001 0 0 0
cpu7 1118264 821 174536 14959651 14820 0 26297 0 0 0
";
    let (_, stat) = parse_stat(proc_stat).unwrap();
    assert_eq!(
        stat,
        Stat {
            user: 9701702,
            nice: 6293,
            system: 1291945,
            idle: 119400172,
            iowait: 120770,
            irq: 0,
            softirq: 120369,
            steal: 0,
            guest: 0,
            guest_nice: 0
        }
    );
}

#[test]
fn parse_proc_uptime() {
    let proc_uptime = "605581.79 954456.53";
    let (_, uptime) = parse_uptime(proc_uptime).unwrap();
    assert_eq!(uptime, Duration::from_secs_f64(605581.79))
}

#[test]
fn parse_proc_meminfo() {
    let proc_meminfo = "MemTotal:       16107060 kB
MemFree:          196332 kB
MemAvailable:   12074844 kB
Buffers:         2756320 kB
Cached:          9002228 kB
SwapCached:        18052 kB
Active:          7307032 kB
";
    let (_, meminfo) = parse_meminfo(proc_meminfo).unwrap();
    assert_eq!(
        meminfo,
        MemInfo {
            total: 16107060,
            available: 12074844,
        }
    );
}

#[test]
fn parse_proc_swaps() {
    let proc_swaps = "Filename				Type		Size		Used		Priority
/swapfi-1                               file		1000000		50000		-2
/swapfi-2                               file		2000000		80000		-2
";
    let (_, swaps) = parse_swaps(proc_swaps).unwrap();
    assert_eq!(
        swaps,
        HashMap::from([
            (
                "/swapfi-1".to_string(),
                Swap {
                    size: 1000000,
                    used: 50000
                }
            ),
            (
                "/swapfi-2".to_string(),
                Swap {
                    size: 2000000,
                    used: 80000,
                }
            )
        ])
    );
}

#[test]
fn test_cpu_idle() {
    let stat = Stat {
        user: 100,
        nice: 200,
        system: 300,
        idle: 4500,
        iowait: 400,
        irq: 500,
        softirq: 600,
        steal: 700,
        guest: 800,
        guest_nice: 900,
    };
    assert_eq!(stat.average_idle(), 50);
}

#[test]
fn test_total_mem_used() {
    let mem = MemInfo {
        total: 1000,
        available: 500,
    };
    assert_eq!(mem.total_used(), 50);
}

#[test]
fn test_total_swap_used() {
    let swap = Swap {
        size: 1000,
        used: 500,
    };
    assert_eq!(swap.total_used(), 50);
}
