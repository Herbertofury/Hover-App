#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverState {
    Dormant,
    ForegroundEligible,
    HoverCandidate,
    DecodePending,
    PreviewVisible,
    PreviewLocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroIdleStatus {
    Pass,
    Defect,
    ExceptionDocumented,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceCounters {
    pub active_timers: u32,
    pub decode_jobs: u32,
    pub render_presents: u64,
    pub wheel_hooks: u32,
    pub raw_input_captures: u32,
    pub disk_reads_after_settle: u64,
    pub network_connections: u32,
    pub worker_threads_awake: u32,
    pub bitmap_uploads: u64,
    pub cache_bytes: usize,
}

impl ResourceCounters {
    pub fn zero_work_counters_are_zero(self) -> bool {
        self.active_timers == 0
            && self.decode_jobs == 0
            && self.render_presents == 0
            && self.wheel_hooks == 0
            && self.raw_input_captures == 0
            && self.disk_reads_after_settle == 0
            && self.network_connections == 0
            && self.worker_threads_awake == 0
            && self.bitmap_uploads == 0
    }
}

impl HoverState {
    pub fn zero_idle_status(
        self,
        counters: ResourceCounters,
        exception_documented: bool,
    ) -> ZeroIdleStatus {
        if self != HoverState::Dormant {
            return ZeroIdleStatus::Defect;
        }
        if counters.zero_work_counters_are_zero() {
            ZeroIdleStatus::Pass
        } else if exception_documented {
            ZeroIdleStatus::ExceptionDocumented
        } else {
            ZeroIdleStatus::Defect
        }
    }

    pub fn allows_cursor_timer(self) -> bool {
        matches!(
            self,
            HoverState::ForegroundEligible
                | HoverState::HoverCandidate
                | HoverState::DecodePending
                | HoverState::PreviewVisible
                | HoverState::PreviewLocked
        )
    }

    pub fn allows_decode(self) -> bool {
        matches!(
            self,
            HoverState::DecodePending | HoverState::PreviewVisible | HoverState::PreviewLocked
        )
    }

    pub fn allows_wheel_capture(self) -> bool {
        matches!(self, HoverState::PreviewVisible | HoverState::PreviewLocked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dormant_zero_counters_pass() {
        assert_eq!(
            HoverState::Dormant.zero_idle_status(ResourceCounters::default(), false),
            ZeroIdleStatus::Pass
        );
    }

    #[test]
    fn dormant_non_zero_counter_is_defect_without_exception() {
        let counters = ResourceCounters {
            active_timers: 1,
            ..Default::default()
        };
        assert_eq!(
            HoverState::Dormant.zero_idle_status(counters, false),
            ZeroIdleStatus::Defect
        );
    }
}
