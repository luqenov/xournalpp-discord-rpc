use std::fs;
use std::path::Path;

pub fn find_start(preferred_pid: Option<u32>, verbose: bool) -> Option<i64> {
    let btime = boot_time()?;

    if let Some(pid) = preferred_pid {
        if let Some(started) = start_time(pid, btime) {
            return Some(started);
        }
    }

    let mut newest: Option<(i64, u32)> = None;
    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        if Some(pid) == preferred_pid {
            continue;
        }
        if let Some(started) = start_time(pid, btime) {
            if newest.map_or(true, |(best, _)| started > best) {
                newest = Some((started, pid));
            }
        }
    }

    if verbose {
        if let Some((_, pid)) = newest {
            eprintln!("xournal-rpc: using Xournal++ pid {pid}");
        }
    }

    newest.map(|(started, _)| started)
}

fn boot_time() -> Option<i64> {
    fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime ")?.trim().parse::<i64>().ok())
}

fn start_time(pid: u32, btime: i64) -> Option<i64> {
    let dir = Path::new("/proc").join(pid.to_string());
    let comm = fs::read_to_string(dir.join("comm")).ok()?;
    if !is_xournalpp_comm(comm.trim()) {
        return None;
    }

    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    let (ticks, state) = parse_start_time(&stat)?;
    if state == 'Z' {
        return None;
    }

    let hz = clock_ticks()?;
    Some(btime + ticks / hz)
}

pub fn is_xournalpp_comm(comm: &str) -> bool {
    comm == "xournalpp" || comm.starts_with("xournalpp.")
}

fn clock_ticks() -> Option<i64> {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    (hz > 0).then_some(hz as i64)
}

fn parse_start_time(stat: &str) -> Option<(i64, char)> {
    let rest = stat.rsplit_once(')')?.1;
    let mut fields = rest.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let ticks = fields.nth(18)?.parse::<i64>().ok()?;
    Some((ticks, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_start_time_field() {
        let stat = "1234 (xournalpp) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19";
        assert_eq!(parse_start_time(stat), Some((19, 'S')));
    }

    #[test]
    fn handles_parentheses_in_comm() {
        let stat = "1234 (xournal (pp)) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19";
        assert_eq!(parse_start_time(stat), Some((19, 'R')));
    }

    #[test]
    fn recognizes_xournalpp() {
        assert!(is_xournalpp_comm("xournalpp"));
        assert!(is_xournalpp_comm("xournalpp.bin"));
        assert!(!is_xournalpp_comm("xournalpp-helper"));
        assert!(!is_xournalpp_comm("firefox"));
    }
}
