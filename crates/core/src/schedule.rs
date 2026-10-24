//! Calendar recurrence in the service's local timezone. Missed runs coalesce into one action.
use crate::*;
use chrono::{DateTime, Datelike, Days, Duration, Local, NaiveDate, NaiveDateTime, TimeZone};

/// Advance before executing: a restart must not repeat a destructive temporary operation.
/// `anchor_at` preserves the original day/time across short months and DST gaps.
pub fn advance(schedule: &mut Schedule, now: i64) -> Result<()> {
    advance_in(schedule, now, &Local)
}
fn advance_in<T: TimeZone>(schedule: &mut Schedule, now: i64, tz: &T) -> Result<()> {
    if schedule.repeat == Repeat::Once {
        schedule.enabled = false;
        return Ok(());
    }
    let anchor = schedule.anchor_at.unwrap_or(schedule.next_at);
    let first = tz.timestamp_opt(anchor, 0).single().ok_or_else(invalid)?;
    let current = tz.timestamp_opt(now, 0).single().ok_or_else(invalid)?;
    let start = first.date_naive();
    let today = current.date_naive();
    // Start close to now even after years offline. At most a few calendar candidates remain.
    let mut index = match schedule.repeat {
        Repeat::Monthly => ((today.year() - start.year()) * 12 + today.month() as i32
            - start.month() as i32)
            .max(0) as u32,
        Repeat::Yearly => (today.year() - start.year()).max(0) as u32,
        _ => today.signed_duration_since(start).num_days().max(0) as u32 / step(schedule.repeat),
    };
    for _ in 0..10 {
        let date = match schedule.repeat {
            Repeat::Monthly => month_date(start, index),
            Repeat::Yearly => month_date(start, index.checked_mul(12).ok_or_else(invalid)?),
            _ => start.checked_add_days(Days::new(
                u64::from(index) * u64::from(step(schedule.repeat)),
            )),
        }
        .ok_or_else(invalid)?;
        if schedule.repeat != Repeat::Weekdays || date.weekday().number_from_monday() <= 5 {
            let next = local_time(tz, date.and_time(first.time()))?;
            if next.timestamp() > now {
                schedule.anchor_at = Some(anchor);
                schedule.next_at = next.timestamp();
                return Ok(());
            }
        }
        index = index.checked_add(1).ok_or_else(invalid)?;
    }
    Err(invalid())
}
fn step(repeat: Repeat) -> u32 {
    match repeat {
        Repeat::Weekly => 7,
        Repeat::Biweekly => 14,
        _ => 1,
    }
}
fn month_date(start: NaiveDate, months: u32) -> Option<NaiveDate> {
    let absolute = i64::from(start.year()) * 12 + i64::from(start.month0()) + i64::from(months);
    let year = i32::try_from(absolute.div_euclid(12)).ok()?;
    let month = absolute.rem_euclid(12) as u32 + 1;
    (28..=start.day())
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .or_else(|| NaiveDate::from_ymd_opt(year, month, start.day()))
}
fn local_time<T: TimeZone>(tz: &T, mut local: NaiveDateTime) -> Result<DateTime<T>> {
    // Nonexistent spring-forward times run at the first valid minute after the gap.
    // Repeated autumn times choose the first occurrence, once per calendar day.
    for _ in 0..=180 {
        if let Some(time) = tz.from_local_datetime(&local).earliest() {
            return Ok(time);
        }
        local = local
            .checked_add_signed(Duration::minutes(1))
            .ok_or_else(invalid)?;
    }
    Err(invalid())
}
fn invalid() -> Error {
    Error::ConfigurationError("schedule outside supported calendar range".into())
}

pub fn command(action: &ScheduleAction) -> Command {
    match *action {
        ScheduleAction::Limit { stop, start } => Command::Limit {
            stop: Some(stop),
            start,
        },
        ScheduleAction::TopUp => Command::TopUp,
        ScheduleAction::ChargeTo { target } => Command::ChargeTo { target },
        ScheduleAction::Discharge { target } => Command::Discharge { target },
        ScheduleAction::Hold => Command::StopCharging,
        ScheduleAction::Pause => Command::Pause,
        ScheduleAction::Resume => Command::Resume,
        ScheduleAction::Calibrate => Command::Calibrate,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, Timelike};
    fn entry(at: &str, repeat: Repeat) -> Schedule {
        Schedule {
            id: 1,
            enabled: true,
            next_at: DateTime::parse_from_rfc3339(at).unwrap().timestamp(),
            anchor_at: None,
            repeat,
            action: ScheduleAction::TopUp,
        }
    }
    #[test]
    fn month_end_and_leap_year_do_not_drift() {
        let tz = FixedOffset::east_opt(3600).unwrap();
        let mut task = entry("2028-01-31T09:30:00+01:00", Repeat::Monthly);
        advance_in(&mut task, task_time("2028-01-31T09:30:00+01:00"), &tz).unwrap();
        assert_eq!(tz.timestamp_opt(task.next_at, 0).unwrap().day(), 29);
        let now = task.next_at;
        advance_in(&mut task, now, &tz).unwrap();
        assert_eq!(tz.timestamp_opt(task.next_at, 0).unwrap().day(), 31);
        let mut annual = entry("2028-02-29T09:30:00+01:00", Repeat::Yearly);
        advance_in(&mut annual, task_time("2031-03-01T00:00:00+01:00"), &tz).unwrap();
        assert_eq!(
            tz.timestamp_opt(annual.next_at, 0).unwrap().date_naive(),
            NaiveDate::from_ymd_opt(2032, 2, 29).unwrap()
        );
    }
    fn task_time(at: &str) -> i64 {
        DateTime::parse_from_rfc3339(at).unwrap().timestamp()
    }
    #[test]
    fn missed_runs_coalesce_and_weekends_are_skipped() {
        let tz = FixedOffset::east_opt(0).unwrap();
        let mut task = entry("2026-10-23T09:30:00Z", Repeat::Weekdays);
        advance_in(&mut task, task_time("2026-10-24T12:00:00Z"), &tz).unwrap();
        let next = tz.timestamp_opt(task.next_at, 0).unwrap();
        assert_eq!(next.day(), 26);
        assert_eq!(next.hour(), 9);
        task.repeat = Repeat::Once;
        advance_in(&mut task, next.timestamp(), &tz).unwrap();
        assert!(!task.enabled);
    }
    // Synthetic timezone exercises DST without changing process-global TZ or adding chrono-tz.
    #[derive(Clone, Copy)]
    struct TestZone;
    impl TimeZone for TestZone {
        type Offset = FixedOffset;
        fn from_offset(_: &FixedOffset) -> Self {
            Self
        }
        fn offset_from_local_date(&self, date: &NaiveDate) -> chrono::MappedLocalTime<FixedOffset> {
            self.offset_from_local_datetime(&date.and_hms_opt(12, 0, 0).unwrap())
        }
        fn offset_from_local_datetime(
            &self,
            at: &NaiveDateTime,
        ) -> chrono::MappedLocalTime<FixedOffset> {
            use chrono::LocalResult::*;
            let winter = FixedOffset::east_opt(3600).unwrap();
            let summer = FixedOffset::east_opt(7200).unwrap();
            let date = (at.month(), at.day());
            if date == (3, 29) && at.hour() == 2 {
                return None;
            }
            if date == (10, 25) && at.hour() == 2 {
                return Ambiguous(summer, winter);
            }
            Single(
                if (date > (3, 29) || date == (3, 29) && at.hour() >= 3)
                    && (date < (10, 25) || date == (10, 25) && at.hour() < 2)
                {
                    summer
                } else {
                    winter
                },
            )
        }
        fn offset_from_utc_date(&self, date: &NaiveDate) -> FixedOffset {
            self.offset_from_utc_datetime(&date.and_hms_opt(12, 0, 0).unwrap())
        }
        fn offset_from_utc_datetime(&self, at: &NaiveDateTime) -> FixedOffset {
            let stamp = (at.month(), at.day(), at.hour());
            FixedOffset::east_opt(if ((3, 29, 1)..(10, 25, 1)).contains(&stamp) {
                7200
            } else {
                3600
            })
            .unwrap()
        }
    }
    #[test]
    fn daylight_saving_gaps_and_folds_run_once_and_preserve_original_wall_time() {
        let mut spring = entry("2026-03-28T02:30:00+01:00", Repeat::Daily);
        let now = spring.next_at;
        advance_in(&mut spring, now, &TestZone).unwrap();
        assert_eq!(spring.next_at, task_time("2026-03-29T03:00:00+02:00"));
        let now = spring.next_at;
        advance_in(&mut spring, now, &TestZone).unwrap();
        assert_eq!(spring.next_at, task_time("2026-03-30T02:30:00+02:00"));
        let mut autumn = entry("2026-10-24T02:30:00+02:00", Repeat::Daily);
        let now = autumn.next_at;
        advance_in(&mut autumn, now, &TestZone).unwrap();
        assert_eq!(autumn.next_at, task_time("2026-10-25T02:30:00+02:00"));
        let now = autumn.next_at;
        advance_in(&mut autumn, now, &TestZone).unwrap();
        assert_eq!(autumn.next_at, task_time("2026-10-26T02:30:00+01:00"));
    }
}
