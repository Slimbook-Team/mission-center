#[derive(Default, Debug, Eq, PartialEq)]
pub struct MemInfo {
    pub mem_total: usize,
    pub mem_free: usize,
    pub mem_available: usize,
    pub buffers: usize,
    pub cached: usize,
    pub swap_cached: usize,
    pub active: usize,
    pub inactive: usize,
    pub active_anon: usize,
    pub inactive_anon: usize,
    pub active_file: usize,
    pub inactive_file: usize,
    pub unevictable: usize,
    pub m_locked: usize,
    pub swap_total: usize,
    pub swap_free: usize,
    pub zswap: usize,
    pub zswapped: usize,
    pub dirty: usize,
    pub writeback: usize,
    pub anon_pages: usize,
    pub mapped: usize,
    pub sh_mem: usize,
    pub k_reclaimable: usize,
    pub slab: usize,
    pub s_reclaimable: usize,
    pub s_unreclaim: usize,
    pub kernel_stack: usize,
    pub page_tables: usize,
    pub sec_page_tables: usize,
    pub nfs_unstable: usize,
    pub bounce: usize,
    pub writeback_tmp: usize,
    pub commit_limit: usize,
    pub committed_as: usize,
    pub vmalloc_total: usize,
    pub vmalloc_used: usize,
    pub vmalloc_chunk: usize,
    pub percpu: usize,
    pub hardware_corrupted: usize,
    pub anon_huge_pages: usize,
    pub shmem_huge_pages: usize,
    pub shmem_pmd_mapped: usize,
    pub file_huge_pages: usize,
    pub file_pmd_mapped: usize,
    pub cma_total: usize,
    pub cma_free: usize,
    pub huge_pages_total: usize,
    pub huge_pages_free: usize,
    pub huge_pages_rsvd: usize,
    pub huge_pages_surp: usize,
    pub hugepagesize: usize,
    pub hugetlb: usize,
    pub direct_map4k: usize,
    pub direct_map2m: usize,
    pub direct_map1g: usize,
}

impl MemInfo {
    pub fn new() -> Self {
        let mut this: Self = Default::default();
        this.refresh();

        this
    }

    pub fn refresh(&mut self) {
        use gtk::glib::*;

        let is_flatpak = *super::IS_FLATPAK;
        let meminfo = if !is_flatpak {
            match std::fs::read_to_string("/proc/meminfo") {
                Ok(meminfo) => meminfo,
                Err(err) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Failed to refresh memory information, failed to read from file: {:?}",
                        err
                    );
                    return;
                }
            }
        } else {
            let mut cmd = std::process::Command::new(super::FLATPAK_SPAWN_CMD);
            cmd.arg("--host")
                .arg("sh")
                .arg("-c")
                .arg("cat /proc/meminfo");

            if let Ok(output) = cmd.output() {
                if output.stderr.len() > 0 {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Failed to refresh memory information, host command execution failed: {}",
                        std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                    );
                    return;
                }

                match std::str::from_utf8(output.stdout.as_slice()) {
                    Ok(out) => out.to_owned(),
                    Err(err) => {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Failed to refresh memory information, host command execution failed: {:?}",
                            err
                        );
                        return;
                    }
                }
            } else {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to refresh memory information, host command execution failed"
                );

                return;
            }
        };

        for line in meminfo.trim().lines() {
            let mut split = line.split_whitespace();
            let key = split.next().map_or("", |s| s);
            let value = split
                .next()
                .map_or("", |s| s)
                .parse::<usize>()
                .map_or(0, |v| v)
                * 1024;

            match key {
                "MemTotal:" => self.mem_total = value,
                "MemFree:" => self.mem_free = value,
                "MemAvailable:" => self.mem_available = value,
                "Buffers:" => self.buffers = value,
                "Cached:" => self.cached = value,
                "SwapCached:" => self.swap_cached = value,
                "Active:" => self.active = value,
                "Inactive:" => self.inactive = value,
                "Active(anon):" => self.active_anon = value,
                "Inactive(anon):" => self.inactive_anon = value,
                "Active(file):" => self.active_file = value,
                "Inactive(file):" => self.inactive_file = value,
                "Unevictable:" => self.unevictable = value,
                "Mlocked:" => self.m_locked = value,
                "SwapTotal:" => self.swap_total = value,
                "SwapFree:" => self.swap_free = value,
                "ZSwap:" => self.zswap = value,
                "ZSwapTotal:" => self.zswapped = value,
                "Dirty:" => self.dirty = value,
                "Writeback:" => self.writeback = value,
                "AnonPages:" => self.anon_pages = value,
                "Mapped:" => self.mapped = value,
                "Shmem:" => self.sh_mem = value,
                "KReclaimable:" => self.k_reclaimable = value,
                "Slab:" => self.slab = value,
                "SReclaimable:" => self.s_reclaimable = value,
                "SUnreclaim:" => self.s_unreclaim = value,
                "KernelStack:" => self.kernel_stack = value,
                "PageTables:" => self.page_tables = value,
                "SecMemTables:" => self.sec_page_tables = value,
                "NFS_Unstable:" => self.nfs_unstable = value,
                "Bounce:" => self.bounce = value,
                "WritebackTmp:" => self.writeback_tmp = value,
                "CommitLimit:" => self.commit_limit = value,
                "Committed_AS:" => self.committed_as = value,
                "VmallocTotal:" => self.vmalloc_total = value,
                "VmallocUsed:" => self.vmalloc_used = value,
                "VmallocChunk:" => self.vmalloc_chunk = value,
                "Percpu:" => self.percpu = value,
                "HardwareCorrupted:" => self.hardware_corrupted = value,
                "AnonHugePages:" => self.anon_huge_pages = value,
                "ShmemHugePages:" => self.shmem_huge_pages = value,
                "ShmemPmdMapped:" => self.shmem_pmd_mapped = value,
                "FileHugePages:" => self.file_huge_pages = value,
                "FilePmdMapped:" => self.file_pmd_mapped = value,
                "CmaTotal:" => self.cma_total = value,
                "CmaFree:" => self.cma_free = value,
                "HugePages_Total:" => self.huge_pages_total = value / 1024,
                "HugePages_Free:" => self.huge_pages_free = value / 1024,
                "HugePages_Rsvd:" => self.huge_pages_rsvd = value / 1024,
                "HugePages_Surp:" => self.huge_pages_surp = value / 1024,
                "Hugepagesize:" => self.hugepagesize = value,
                "Hugetlb:" => self.hugetlb = value,
                "DirectMap4k:" => self.direct_map4k = value,
                "DirectMap2M:" => self.direct_map2m = value,
                "DirectMap1G:" => self.direct_map1g = value,
                _ => (),
            }
        }
    }
}
