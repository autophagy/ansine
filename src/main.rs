use std::str;
use std::time::Duration;

use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{digit1, multispace0},
    combinator::{map, map_res},
    error::ParseError,
    number::complete::double,
    sequence::{delimited, preceded, tuple},
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

fn parse_uptime(i: &str) -> IResult<&str, Duration> {
    let (i, u) = parse_f64(i)?;
    Ok((i, Duration::from_secs_f64(u)))
}

fn main() {}

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
