use nom::{
    bytes::complete::{tag, take_till, take_until},
    character::complete::{alphanumeric1, char, digit1, line_ending, multispace0},
    combinator::{map, map_res, opt, recognize, rest},
    error::ParseError,
    multi::many0,
    number::complete::double,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use std::collections::HashMap;
use std::str;
use std::time::Duration;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Stat {
    pub user: usize,
    pub nice: usize,
    pub system: usize,
    pub idle: usize,
    pub iowait: usize,
    pub irq: usize,
    pub softirq: usize,
    pub steal: usize,
    pub guest: usize,
    pub guest_nice: usize,
}

pub type MemInfo = HashMap<String, usize>;

#[derive(Debug, PartialEq)]
enum SwapType {
    File,
    Partition,
}

#[derive(Debug, PartialEq, Eq)]
struct ParseSwapTypeError;

impl str::FromStr for SwapType {
    type Err = ParseSwapTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" => Ok(SwapType::File),
            "partition" => Ok(SwapType::Partition),
            _ => Err(ParseSwapTypeError),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Swap {
    swap_type: SwapType,
    pub size: usize,
    pub used: usize,
    priority: isize,
}

pub type Swaps = HashMap<String, Swap>;

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
        recognize(preceded(opt(char('-')), digit1)),
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
    map(many0(parse_meminfo_line), |meminfo| {
        meminfo.into_iter().collect()
    })(i)
}

fn parse_meminfo_line(i: &str) -> IResult<&str, (String, usize)> {
    let (i, (meminfo, size)) = tuple((
        take_until(":"),
        delimited(
            tag(":"),
            parse_usize,
            terminated(opt(tag("kB")), opt(line_ending)),
        ),
    ))(i)?;
    Ok((i, (meminfo.to_string(), size)))
}

pub fn parse_swaps(i: &str) -> IResult<&str, Swaps> {
    let (i, _) = tuple((
        ws(tag("Filename")),
        ws(tag("Type")),
        ws(tag("Size")),
        ws(tag("Used")),
        ws(terminated(tag("Priority"), line_ending)),
    ))(i)?;

    map(many0(parse_swap_line), |swaps| swaps.into_iter().collect())(i)
}

pub fn parse_swap_line(i: &str) -> IResult<&str, (String, Swap)> {
    let (i, filename) = take_till(char::is_whitespace)(i)?;
    let (i, swap_type) = map_res(ws(alphanumeric1), str::FromStr::from_str)(i)?;
    let (i, (size, used, priority)) =
        terminated(tuple((parse_usize, parse_usize, parse_isize)), line_ending)(i)?;
    Ok((
        i,
        (
            filename.to_string(),
            Swap {
                swap_type,
                size,
                used,
                priority,
            },
        ),
    ))
}

pub fn parse_nix_store_path(i: &str) -> IResult<&str, &str> {
    preceded(tag("/nix/store/"), rest)(i)
}

#[cfg(test)]
mod parsing_tests {
    use super::*;

    #[test]
    fn proc_stat() {
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
    fn proc_uptime() {
        let proc_uptime = "605581.79 954456.53";
        let (_, uptime) = parse_uptime(proc_uptime).unwrap();
        assert_eq!(uptime, Duration::from_secs_f64(605581.79))
    }

    #[test]
    fn proc_meminfo() {
        let proc_meminfo = "MemTotal:       16107060 kB
MemFree:         1916068 kB
MemAvailable:   11569620 kB
HugePages_Total:       0
HugePages_Free:        0
DirectMap1G:     4194304 kB";
        let (_, meminfo) = parse_meminfo(proc_meminfo).unwrap();
        assert_eq!(
            meminfo,
            HashMap::from([
                ("MemTotal".to_string(), 16107060),
                ("MemFree".to_string(), 1916068),
                ("MemAvailable".to_string(), 11569620),
                ("HugePages_Total".to_string(), 0),
                ("HugePages_Free".to_string(), 0),
                ("DirectMap1G".to_string(), 4194304),
            ])
        );
    }

    #[test]
    fn proc_swaps() {
        let proc_swaps = "Filename				Type		Size		Used		Priority
/swapfile                               file		1000000		50000		-2
/swappart                               partition	2000000		80000		-2
";
        let (_, swaps) = parse_swaps(proc_swaps).unwrap();
        assert_eq!(
            swaps,
            HashMap::from([
                (
                    "/swapfile".to_string(),
                    Swap {
                        swap_type: SwapType::File,

                        size: 1000000,
                        used: 50000,
                        priority: -2,
                    }
                ),
                (
                    "/swappart".to_string(),
                    Swap {
                        swap_type: SwapType::Partition,
                        size: 2000000,
                        used: 80000,
                        priority: -2
                    }
                )
            ])
        );
    }

    #[test]
    fn nix_store_path() {
        let store_path =
        "/nix/store/072jh6kxgpr04zbdqsy1isbrz5xbkcmb-nixos-system-heorot-23.05.20221218.04f574a";
        let (_, path) = parse_nix_store_path(store_path).unwrap();
        assert_eq!(
            path,
            "072jh6kxgpr04zbdqsy1isbrz5xbkcmb-nixos-system-heorot-23.05.20221218.04f574a"
        );
    }
}
