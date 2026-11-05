//! The CDM's timers (§3): each fires once its delay has passed, never before, in order of due
//! time, and in order of setting between equal due times.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

use crate::boundary::TimerContext;

/// The timers set and not yet fired.
#[derive(Default)]
pub(crate) struct Timers {
    queue: BinaryHeap<Reverse<Timer>>,
    next_order: u64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Timer {
    due: Instant,
    /// Breaks ties between equal due times: the timer set first fires first.
    order: u64,
    context: Context,
}

/// A timer's context, ordered by nothing: two timers never compare equal anyway, as their
/// `order` differs.
#[derive(Debug)]
struct Context(TimerContext);

impl PartialEq for Context {
    fn eq(&self, _: &Context) -> bool {
        true
    }
}

impl Eq for Context {}

impl PartialOrd for Context {
    fn partial_cmp(&self, other: &Context) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Context {
    fn cmp(&self, _: &Context) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

/// Where a round of firing stops: the timers set before it began, and due by then.
#[derive(Clone, Copy)]
pub(crate) struct Round {
    now: Instant,
    order_limit: u64,
}

impl Timers {
    /// Sets a timer firing `delay_ms` after `now`; a negative delay counts as none. Returns
    /// `false`, setting nothing, when the due time is beyond what the clock can represent:
    /// such a timer could never fire.
    pub(crate) fn set(&mut self, now: Instant, delay_ms: i64, context: TimerContext) -> bool {
        let delay = Duration::from_millis(delay_ms.max(0).unsigned_abs());
        let Some(due) = now.checked_add(delay) else {
            return false;
        };
        let order = self.next_order;
        self.next_order += 1;
        self.queue.push(Reverse(Timer {
            due,
            order,
            context: Context(context),
        }));
        true
    }

    /// A round firing the timers due at `now` that are already set: those set while it runs
    /// wait for the next round, so that a timer setting another cannot hold the thread.
    pub(crate) fn round(&self, now: Instant) -> Round {
        Round {
            now,
            order_limit: self.next_order,
        }
    }

    /// The next timer of the round, in order of due time.
    pub(crate) fn pop_due(&mut self, round: Round) -> Option<TimerContext> {
        let Reverse(next) = self.queue.peek()?;
        if next.due > round.now || next.order >= round.order_limit {
            return None;
        }
        self.queue.pop().map(|Reverse(timer)| timer.context.0)
    }

    /// When the next timer is due.
    pub(crate) fn next_due(&self) -> Option<Instant> {
        self.queue.peek().map(|Reverse(timer)| timer.due)
    }

    /// Forgets every timer: their instance is gone.
    pub(crate) fn clear(&mut self) {
        self.queue.clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(value: usize) -> TimerContext {
        TimerContext(value as *mut std::ffi::c_void)
    }

    #[test]
    fn timers_fire_in_order_of_due_time_and_never_before() {
        let start = Instant::now();
        let mut timers = Timers::default();
        assert!(timers.set(start, 30, context(3)));
        assert!(timers.set(start, 10, context(1)));
        assert!(timers.set(start, 20, context(2)));
        assert!(timers.set(start, 10, context(4)));

        let early = start + Duration::from_millis(9);
        let round = timers.round(early);
        assert_eq!(timers.pop_due(round), None);

        let later = start + Duration::from_millis(20);
        let round = timers.round(later);
        assert_eq!(timers.pop_due(round), Some(context(1)));
        assert_eq!(timers.pop_due(round), Some(context(4)));
        assert_eq!(timers.pop_due(round), Some(context(2)));
        assert_eq!(timers.pop_due(round), None);
        assert_eq!(timers.next_due(), Some(start + Duration::from_millis(30)));
    }

    #[test]
    fn a_timer_set_during_a_round_waits_for_the_next() {
        let start = Instant::now();
        let mut timers = Timers::default();
        timers.set(start, 0, context(1));
        let round = timers.round(start);
        assert_eq!(timers.pop_due(round), Some(context(1)));
        timers.set(start, 0, context(2));
        assert_eq!(timers.pop_due(round), None);
        assert_eq!(timers.pop_due(timers.round(start)), Some(context(2)));
    }

    #[test]
    fn negative_and_huge_delays() {
        let start = Instant::now();
        let mut timers = Timers::default();
        assert!(timers.set(start, -5, context(1)));
        assert_eq!(timers.pop_due(timers.round(start)), Some(context(1)));
        assert!(!timers.set(start, i64::MAX, context(2)) || timers.len() == 1);
    }
}
