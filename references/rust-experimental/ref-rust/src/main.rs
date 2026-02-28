use std::fs;
use std::thread;
use std::time::Duration;

fn read_uptime() -> u64 {
    fs::read_to_string("/proc/uptime")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|f| f as u64)
        .unwrap_or(0)
}

fn read_meminfo(key: &str) -> u64 {
    fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find(|line| line.starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn read_loadavg() -> String {
    fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or("0.00")
        .to_string()
}

fn read_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string()
}

fn main() {
    let interval = Duration::from_secs(30);
    let hostname = read_hostname();

    loop {
        let uptime = read_uptime();
        let mem_total_kb = read_meminfo("MemTotal:");
        let mem_free_kb = read_meminfo("MemFree:");
        let load_1m = read_loadavg();

        println!(
            r#"{{"hostname":"{}","uptime":{},"mem_total_kb":{},"mem_free_kb":{},"load_1m":"{}"}}"#,
            hostname, uptime, mem_total_kb, mem_free_kb, load_1m
        );

        thread::sleep(interval);
    }
}
