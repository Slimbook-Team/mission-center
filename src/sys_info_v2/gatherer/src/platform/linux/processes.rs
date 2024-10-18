/* sys_info_v2/gatherer/src/platform/linux/processes.rs
 *
 * Copyright 2024 Romeo Calota
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::sync::Arc;
use std::time::Instant;

use lazy_static::lazy_static;

use crate::platform::processes::*;

use super::{HZ, MIN_DELTA_REFRESH};

lazy_static! {
    static ref PAGE_SIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
}

const PROC_PID_STAT_TCOMM: usize = 1;
const PROC_PID_STAT_STATE: usize = 2;
const PROC_PID_STAT_PPID: usize = 3;
const PROC_PID_STAT_UTIME: usize = 13;
const PROC_PID_STAT_STIME: usize = 14;

#[allow(dead_code)]
const PROC_PID_STATM_VIRT: usize = 0;
const PROC_PID_STATM_RES: usize = 1;

const PROC_PID_IO_READ_BYTES: usize = 4;
const PROC_PID_IO_WRITE_BYTES: usize = 5;

#[allow(dead_code)]
const PROC_PID_NET_DEV_RECV_BYTES: usize = 0;
#[allow(dead_code)]
const PROC_PID_NET_DEV_SENT_BYTES: usize = 8;

const STALE_DELTA: std::time::Duration = std::time::Duration::from_millis(1000);

#[derive(Debug, Copy, Clone)]
struct RawStats {
    pub user_jiffies: u64,
    pub kernel_jiffies: u64,

    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,

    pub net_bytes_sent: u64,
    pub net_bytes_recv: u64,

    pub timestamp: Instant,
}

impl Default for RawStats {
    fn default() -> Self {
        Self {
            user_jiffies: 0,
            kernel_jiffies: 0,

            disk_read_bytes: 0,
            disk_write_bytes: 0,

            net_bytes_sent: 0,
            net_bytes_recv: 0,

            timestamp: Instant::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinuxProcess {
    name: Arc<str>,
    cmd: Vec<Arc<str>>,
    exe: Arc<str>,
    state: ProcessState,
    pid: u32,
    parent: u32,
    // Needs to be pub to be accessible from GPU info
    pub usage_stats: ProcessUsageStats,
    task_count: usize,

    pub cgroup: Option<Arc<str>>,

    raw_stats: RawStats,
}

impl Default for LinuxProcess {
    fn default() -> Self {
        Self {
            name: "".into(),
            cmd: vec![],
            exe: "".into(),
            state: ProcessState::Unknown,
            pid: 0,
            parent: 0,
            usage_stats: Default::default(),
            task_count: 0,
            cgroup: None,
            raw_stats: Default::default(),
        }
    }
}

impl<'a> ProcessExt<'a> for LinuxProcess {
    type Iter = std::iter::Map<std::slice::Iter<'a, Arc<str>>, fn(&'a Arc<str>) -> &'a str>;

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn cmd(&'a self) -> Self::Iter {
        self.cmd.iter().map(|e| e.as_ref())
    }

    fn exe(&self) -> &str {
        self.exe.as_ref()
    }

    fn state(&self) -> ProcessState {
        self.state
    }

    fn pid(&self) -> u32 {
        self.pid
    }

    fn parent(&self) -> u32 {
        self.parent
    }

    fn usage_stats(&self) -> &ProcessUsageStats {
        &self.usage_stats
    }

    fn task_count(&self) -> usize {
        self.task_count
    }
}

pub struct LinuxProcesses {
    process_cache: std::collections::HashMap<u32, LinuxProcess>,
    refresh_timestamp: Instant,
}

impl LinuxProcesses {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for LinuxProcesses {
    fn default() -> Self {
        Self {
            process_cache: std::collections::HashMap::new(),
            refresh_timestamp: std::time::Instant::now()
                - (STALE_DELTA + std::time::Duration::from_millis(1)),
        }
    }
}

impl<'a> ProcessesExt<'a> for LinuxProcesses {
    type P = LinuxProcess;

    fn refresh_cache(&mut self) {
        use crate::{critical, debug, warning};
        use std::io::Read;

        fn parse_stat_file<'a>(data: &'a str, output: &mut [&'a str; 52]) {
            let mut part_index = 0;

            let mut split = data.split('(').filter(|x| !x.is_empty());
            output[part_index] = match split.next() {
                Some(x) => x,
                None => return,
            };
            part_index += 1;

            let mut split = match split.next() {
                Some(x) => x.split(')').filter(|x| !x.is_empty()),
                None => return,
            };

            output[part_index] = match split.next() {
                Some(x) => x,
                None => return,
            };
            part_index += 1;

            let split = match split.next() {
                Some(x) => x,
                None => return,
            };
            for entry in split.split_whitespace() {
                output[part_index] = entry;
                part_index += 1;
            }
        }

        fn parse_statm_file(data: &str, output: &mut [u64; 7]) {
            let mut part_index = 0;

            for entry in data.split_whitespace() {
                output[part_index] = entry.trim().parse::<u64>().unwrap_or(0);
                part_index += 1;
            }
        }

        fn parse_io_file(data: &str, output: &mut [u64; 7]) {
            let mut part_index = 0;

            for entry in data.lines() {
                let entry = entry.split_whitespace().last().unwrap_or("");
                output[part_index] = entry.trim().parse::<u64>().unwrap_or(0);
                part_index += 1;
            }
        }

        fn stat_name(stat: &[&str; 52]) -> Arc<str> {
            stat[PROC_PID_STAT_TCOMM].into()
        }

        fn stat_state(stat: &[&str; 52]) -> ProcessState {
            match stat[PROC_PID_STAT_STATE] {
                "R" => ProcessState::Running,
                "S" => ProcessState::Sleeping,
                "D" => ProcessState::SleepingUninterruptible,
                "Z" => ProcessState::Zombie,
                "T" => ProcessState::Stopped,
                "t" => ProcessState::Tracing,
                "X" | "x" => ProcessState::Dead,
                "K" => ProcessState::WakeKill,
                "W" => ProcessState::Waking,
                "P" => ProcessState::Parked,
                _ => ProcessState::Unknown,
            }
        }

        fn stat_parent_pid(stat: &[&str; 52]) -> u32 {
            stat[PROC_PID_STAT_PPID].parse::<u32>().unwrap_or(0)
        }

        fn stat_user_mode_jiffies(stat: &[&str; 52]) -> u64 {
            stat[PROC_PID_STAT_UTIME].parse::<u64>().unwrap_or(0)
        }

        fn stat_kernel_mode_jiffies(stat: &[&str; 52]) -> u64 {
            stat[PROC_PID_STAT_STIME].parse::<u64>().unwrap_or(0)
        }

        let now = Instant::now();
        if now.duration_since(self.refresh_timestamp) < MIN_DELTA_REFRESH {
            return;
        }
        self.refresh_timestamp = now;

        let mut previous = std::mem::take(&mut self.process_cache);
        let result = &mut self.process_cache;
        result.reserve(previous.len());

        let mut stat_file_content = String::new();
        stat_file_content.reserve(512);

        let mut read_buffer = String::new();
        read_buffer.reserve(512);

        let proc = match std::fs::read_dir("/proc") {
            Ok(proc) => proc,
            Err(e) => {
                critical!(
                    "Gatherer::Processes",
                    "Failed to read /proc directory: {}",
                    e
                );
                return;
            }
        };
        let proc_entries = proc
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false));
        for entry in proc_entries {
            let pid = match entry.file_name().to_string_lossy().parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            let entry_path = entry.path();

            let mut stat_file = match std::fs::OpenOptions::new()
                .read(true)
                .open(entry_path.join("stat"))
            {
                Ok(f) => f,
                Err(e) => {
                    critical!(
                        "Gatherer::Processes",
                        "Failed to read `stat` file for process {}, skipping: {}",
                        pid,
                        e,
                    );
                    continue;
                }
            };
            stat_file_content.clear();
            match stat_file.read_to_string(&mut stat_file_content) {
                Ok(sfc) => {
                    if sfc == 0 {
                        critical!(
                            "Gatherer::Processes",
                            "Failed to read stat information for process {}, skipping",
                            pid
                        );
                        continue;
                    }
                }
                Err(e) => {
                    critical!(
                        "Gatherer::Processes",
                        "Failed to read stat information for process {}, skipping: {}",
                        pid,
                        e,
                    );
                    continue;
                }
            };
            let mut stat_parsed = [""; 52];
            parse_stat_file(&stat_file_content, &mut stat_parsed);

            let utime = stat_user_mode_jiffies(&stat_parsed);
            let stime = stat_kernel_mode_jiffies(&stat_parsed);

            let mut io_parsed = [0; 7];
            match std::fs::OpenOptions::new()
                .read(true)
                .open(entry_path.join("io"))
            {
                Ok(mut f) => {
                    read_buffer.clear();
                    match f.read_to_string(&mut read_buffer) {
                        Ok(_) => {
                            parse_io_file(&read_buffer, &mut io_parsed);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    debug!(
                        "Gatherer::Processes",
                        "Failed to read `io` file for process {}: {}", pid, e,
                    );
                }
            };

            let total_net_sent = 0_u64;
            let total_net_recv = 0_u64;

            let mut process = match previous.remove(&pid) {
                None => LinuxProcess::default(),
                Some(mut process) => {
                    let delta_time = now - process.raw_stats.timestamp;

                    let prev_utime = process.raw_stats.user_jiffies;
                    let prev_stime = process.raw_stats.kernel_jiffies;

                    let delta_utime =
                        ((utime.saturating_sub(prev_utime) as f32) * 1000.) / *HZ as f32;
                    let delta_stime =
                        ((stime.saturating_sub(prev_stime) as f32) * 1000.) / *HZ as f32;

                    process.usage_stats.cpu_usage =
                        (((delta_utime + delta_stime) / delta_time.as_millis() as f32) * 100.)
                            .min((*super::CPU_COUNT as f32) * 100.);

                    let prev_read_bytes = process.raw_stats.disk_read_bytes;
                    let prev_write_bytes = process.raw_stats.disk_write_bytes;

                    let read_speed =
                        io_parsed[PROC_PID_IO_READ_BYTES].saturating_sub(prev_read_bytes) as f32
                            / delta_time.as_secs_f32();
                    let write_speed =
                        io_parsed[PROC_PID_IO_WRITE_BYTES].saturating_sub(prev_write_bytes) as f32
                            / delta_time.as_secs_f32();
                    process.usage_stats.disk_usage = (read_speed + write_speed) / 2.;

                    process
                }
            };

            let cmd = match std::fs::OpenOptions::new()
                .read(true)
                .open(entry_path.join("cmdline"))
            {
                Ok(mut f) => {
                    read_buffer.clear();
                    match f.read_to_string(&mut read_buffer) {
                        Ok(_) => read_buffer
                            .split('\0')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .map(|s| Arc::<str>::from(s))
                            .collect::<Vec<_>>(),
                        Err(e) => {
                            warning!(
                                "Gatherer::Processes",
                                "Failed to parse commandline for {}: {}",
                                pid,
                                e
                            );
                            vec![]
                        }
                    }
                }
                Err(e) => {
                    warning!(
                        "Gatherer::Processes",
                        "Failed to read `cmdline` file for process {}: {}",
                        pid,
                        e,
                    );
                    vec![]
                }
            };

            let output = entry_path.join("exe").read_link();
            let exe = output
                .map(|p| p.as_os_str().to_string_lossy().into())
                .unwrap_or("".into());

            let mut statm_parsed = [0; 7];
            match std::fs::OpenOptions::new()
                .read(true)
                .open(entry_path.join("statm"))
            {
                Ok(mut f) => {
                    read_buffer.clear();
                    match f.read_to_string(&mut read_buffer) {
                        Ok(_) => {
                            parse_statm_file(&read_buffer, &mut statm_parsed);
                        }
                        Err(e) => {
                            warning!(
                                "Gatherer::Processes",
                                "Failed to read memory information for {}: {}",
                                pid,
                                e
                            );
                        }
                    };
                }
                Err(e) => {
                    warning!(
                        "Gatherer::Processes",
                        "Failed to read `statm` file for process {}: {}",
                        pid,
                        e,
                    );
                }
            };

            let cgroup = match std::fs::OpenOptions::new()
                .read(true)
                .open(entry_path.join("cgroup"))
            {
                Ok(mut f) => {
                    read_buffer.clear();
                    match f.read_to_string(&mut read_buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                warning!(
                                    "Gatherer::Processes",
                                    "Failed to read cgroup information for process {}: No cgroup associated with process",
                                    pid
                                );
                                None
                            } else {
                                let mut cgroup = None;

                                let cfc = read_buffer
                                    .trim()
                                    .split(':')
                                    .nth(2)
                                    .unwrap_or("/")
                                    .trim_start_matches('/')
                                    .trim_end_matches(&format!("/{}", pid));

                                let cgroup_path = std::path::Path::new("/sys/fs/cgroup").join(cfc);
                                if !cfc.is_empty() && cgroup_path.exists() && cgroup_path.is_dir() {
                                    let app_scope = cfc.split('/').last().unwrap_or("");
                                    if (app_scope.starts_with("app")
                                        || app_scope.starts_with("snap"))
                                        && app_scope.ends_with(".scope")
                                    {
                                        cgroup = Some(cgroup_path.to_string_lossy().into());
                                    }
                                }

                                cgroup
                            }
                        }
                        Err(e) => {
                            warning!(
                                "Gatherer::Processes",
                                "Failed to read cgroup information for process {}: {}",
                                pid,
                                e
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    warning!(
                        "Gatherer::Processes",
                        "Failed to read `cgroup` file for process {}: {}",
                        pid,
                        e,
                    );
                    None
                }
            };

            let mut task_count = 0_usize;
            match std::fs::read_dir(entry_path.join("task")) {
                Ok(tasks) => {
                    for task in tasks.filter_map(|t| t.ok()) {
                        match task.file_name().to_string_lossy().parse::<u32>() {
                            Err(_) => continue,
                            _ => {}
                        };
                        task_count += 1;
                    }
                }
                Err(e) => {
                    warning!(
                        "Gatherer::Processes",
                        "Gatherer: Failed to read task directory for process {}: {}",
                        pid,
                        e
                    );
                }
            }

            process.pid = pid;
            process.name = stat_name(&stat_parsed);
            process.cmd = cmd;
            process.exe = exe;
            process.state = stat_state(&stat_parsed);
            process.parent = stat_parent_pid(&stat_parsed);
            process.usage_stats.memory_usage =
                (statm_parsed[PROC_PID_STATM_RES] * (*PAGE_SIZE) as u64) as f32;
            process.task_count = task_count;
            process.raw_stats.user_jiffies = utime;
            process.raw_stats.kernel_jiffies = stime;
            process.raw_stats.disk_read_bytes = io_parsed[PROC_PID_IO_READ_BYTES];
            process.raw_stats.disk_write_bytes = io_parsed[PROC_PID_IO_WRITE_BYTES];
            process.raw_stats.net_bytes_sent = total_net_sent;
            process.raw_stats.net_bytes_recv = total_net_recv;
            process.raw_stats.timestamp = now;
            process.cgroup = cgroup;

            result.insert(pid, process);
        }

        self.refresh_timestamp = Instant::now();
    }

    fn process_list(&'a self) -> &'a std::collections::HashMap<u32, LinuxProcess> {
        &self.process_cache
    }

    fn process_list_mut(&mut self) -> &mut std::collections::HashMap<u32, LinuxProcess> {
        &mut self.process_cache
    }

    fn terminate_process(&self, pid: u32) {
        use libc::*;

        unsafe {
            kill(pid as pid_t, SIGTERM);
        }
    }

    fn kill_process(&self, pid: u32) {
        use libc::*;

        unsafe {
            kill(pid as pid_t, SIGKILL);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::platform::ProcessesExt;

    use super::*;

    #[test]
    fn test_refresh_cache() {
        let mut p = LinuxProcesses::new();
        assert!(p.process_cache.is_empty());

        p.refresh_cache();
        assert!(!p.process_cache.is_empty());

        let sample = p
            .process_cache
            .iter()
            .filter(|(_pid, proc)| proc.raw_stats.user_jiffies > 0)
            .map(|p| p.1)
            .take(10);
        dbg!(&sample);
    }
}
