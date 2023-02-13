use std::str;
use std::time::Duration;

use nom::{
    bytes::complete::tag,
    character::complete::{digit1, multispace0},
    combinator::{map, map_res},
    error::ParseError,
    number::complete::double,
    sequence::{delimited, tuple},
    IResult,
};

#[derive(Debug, PartialEq)]
struct Stat {
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

#[derive(Debug, PartialEq)]
struct MemInfo {
    total: usize,
    free: usize,
}

fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_usize(i: &str) -> IResult<&str, usize> {
    let parser = map_res(map(ws(digit1), str::as_bytes), str::from_utf8);
    map_res(parser, str::FromStr::from_str)(i)
}

fn parse_f64(i: &str) -> IResult<&str, f64> {
    ws(double)(i)
}

fn parse_stat(i: &str) -> IResult<&str, Stat> {
    let (i, _) = tag("cpu")(i)?;
    let (i, (user, nice, system, idle, iowait, irq, softirq, steal, guest, guest_nice)) = tuple((
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
    ))(i)?;
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

fn parse_uptime(i: &str) -> IResult<&str, Duration> {
    let (i, u) = parse_f64(i)?;
    Ok((i, Duration::from_secs_f64(u)))
}

fn main() {}

#[test]
fn parse_proc_stat() {
    let proc_stat = "cpu  8934555 4605 1022996 89784995 91669 0 89602 0 0 0";
    let (_, stat) = parse_stat(proc_stat).unwrap();
    assert_eq!(
        stat,
        Stat {
            user: 8934555,
            nice: 4605,
            system: 1022996,
            idle: 89784995,
            iowait: 91669,
            irq: 0,
            softirq: 89602,
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
