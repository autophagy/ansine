use std::{fs::read_to_string, net::SocketAddr, str, time::Duration};

use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{digit1, multispace0},
    combinator::{map, map_res},
    error::ParseError,
    number::complete::double,
    sequence::{delimited, preceded, tuple},
    IResult,
};

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
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
    available: usize,
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

fn parse_meminfo(i: &str) -> IResult<&str, MemInfo> {
    let (i, total) = delimited(tag("MemTotal:"), parse_usize, tag("kB\n"))(i)?;
    let (i, _) = delimited(tag("MemFree:"), parse_usize, tag("kB\n"))(i)?;
    let (i, available) = delimited(tag("MemAvailable:"), parse_usize, tag("kB\n"))(i)?;
    Ok((i, MemInfo { total, available }))
}

fn average_cpu_idle(cpu: &Stat) -> usize {
    (cpu.idle * 100)
        / (cpu.user
            + cpu.nice
            + cpu.system
            + cpu.idle
            + cpu.iowait
            + cpu.irq
            + cpu.softirq
            + cpu.steal
            + cpu.guest
            + cpu.guest_nice)
}

fn total_used_memory(mem: &MemInfo) -> usize {
    ((mem.total - mem.available) * 100) / mem.total
}

fn read_file(fp: &str) -> String {
    read_to_string(fp).expect("Unable to read file")
}

fn format_duration(duration: &Duration) -> String {
    let secs = duration.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("{}d.{}h.{}m", days, hours, mins)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(root));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    uptime: String,
    cpu: String,
    mem: String,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

fn progress_bar(value: usize, max: usize) -> String {
    let segment_value = max / 20;
    let segments = value / segment_value;
    format!("[{}{}]", "#".repeat(segments), ":".repeat(20 - segments))
}

async fn root() -> impl IntoResponse {
    let proc_stat = read_file("/proc/stat");
    let proc_meminfo = read_file("/proc/meminfo");
    let proc_uptime = read_file("/proc/uptime");

    let (_, stat) = parse_stat(&proc_stat).expect("Unable to parse /proc/stat");
    let (_, mem_info) = parse_meminfo(&proc_meminfo).expect("Unable to parse /proc/meminfo");
    let (_, uptime) = parse_uptime(&proc_uptime).expect("Unable to parse /proc/uptime");

    let cpu_bar = progress_bar(100 - average_cpu_idle(&stat), 100);
    let mem_bar = progress_bar(total_used_memory(&mem_info), 100);

    let template = IndexTemplate {
        uptime: format_duration(&uptime),
        cpu: cpu_bar,
        mem: mem_bar,
    };
    HtmlTemplate(template)
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
    assert_eq!(average_cpu_idle(&stat), 50);
}

#[test]
fn test_total_mem_used() {
    let mem = MemInfo {
        total: 1000,
        available: 500,
    };
    assert_eq!(total_used_memory(&mem), 50);
}
