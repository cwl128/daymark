use crate::model::Item;
use chrono::{Days, Duration, Local, NaiveDate, NaiveTime, TimeZone};

pub fn parse_date(value: &str) -> Result<NaiveDate, String> {
    if value.len() != 10 {
        return Err("请选择有效日期".into());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| "请选择有效日期".into())
}
pub fn parse_clock(value: &str) -> Result<NaiveTime, String> {
    if value.len() != 5 {
        return Err("时间请用 24 小时制，例如 21:00".into());
    }
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| "时间请用 24 小时制，例如 21:00".into())
}

// Resolve every date in local wall time, never by adding 86400 seconds. During
// a DST gap use the first valid minute; during a repeated hour use the first.
pub fn on_date<T: TimeZone>(base: &Item, date: NaiveDate, zone: &T) -> Option<Item> {
    let rule = base.recurrence.as_ref()?;
    if date < parse_date(&rule.start_date).ok()? {
        return None;
    }
    let nominal = date.and_time(parse_clock(&rule.time).ok()?);
    let at = (0..=180)
        .map(|minutes| nominal + Duration::minutes(minutes))
        .take_while(|time| time.date() == date)
        .find_map(|time| zone.from_local_datetime(&time).earliest())?;
    let key = date.to_string();
    let state = base.occurrence_states.get(&key);
    let mut item = base.clone();
    item.scheduled_at = Some(at.timestamp());
    item.occurrence_date = Some(key);
    item.completed_at = state.and_then(|s| s.completed_at);
    item.snooze_until = state.and_then(|s| s.snooze_until);
    Some(item)
}

pub fn around_now(items: Vec<Item>, now: i64) -> Vec<Item> {
    let Some(date) =
        chrono::DateTime::from_timestamp(now, 0).map(|t| t.with_timezone(&Local).date_naive())
    else {
        return vec![];
    };
    items
        .into_iter()
        .flat_map(|item| {
            if item.recurrence.is_none() {
                return vec![item];
            }
            [
                date.checked_sub_days(Days::new(1)),
                Some(date),
                date.checked_add_days(Days::new(1)),
            ]
            .into_iter()
            .flatten()
            .filter_map(|date| on_date(&item, date, &Local))
            .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DailyRecurrence;
    fn daily(time: &str) -> Item {
        Item {
            kind: "task".into(),
            title: "和爸妈打电话".into(),
            recurrence: Some(DailyRecurrence {
                start_date: "2026-01-01".into(),
                time: time.into(),
            }),
            ..Default::default()
        }
    }
    #[test]
    fn local_21_stays_21_across_dst_and_uses_calendar_days() {
        let zone = chrono_tz::America::Chicago;
        let before = on_date(&daily("21:00"), parse_date("2027-03-13").unwrap(), &zone).unwrap();
        let after = on_date(&daily("21:00"), parse_date("2027-03-14").unwrap(), &zone).unwrap();
        assert_eq!(
            after.target().unwrap() - before.target().unwrap(),
            23 * 3600
        );
        let local = chrono::DateTime::from_timestamp(after.target().unwrap(), 0)
            .unwrap()
            .with_timezone(&zone);
        assert_eq!(local.format("%H:%M").to_string(), "21:00");
        let before = on_date(&daily("21:00"), parse_date("2026-10-31").unwrap(), &zone).unwrap();
        let after = on_date(&daily("21:00"), parse_date("2026-11-01").unwrap(), &zone).unwrap();
        assert_eq!(
            after.target().unwrap() - before.target().unwrap(),
            25 * 3600
        );
    }
    #[test]
    fn gap_moves_to_first_valid_minute_and_ambiguous_hour_fires_once() {
        let zone = chrono_tz::America::Chicago;
        let gap = on_date(&daily("02:30"), parse_date("2027-03-14").unwrap(), &zone).unwrap();
        let local = chrono::DateTime::from_timestamp(gap.target().unwrap(), 0)
            .unwrap()
            .with_timezone(&zone);
        assert_eq!(local.format("%H:%M").to_string(), "03:00");
        let repeated = on_date(&daily("01:30"), parse_date("2026-11-01").unwrap(), &zone).unwrap();
        assert_eq!(
            repeated.target().unwrap(),
            zone.with_ymd_and_hms(2026, 11, 1, 1, 30, 0)
                .earliest()
                .unwrap()
                .timestamp()
        );
    }
    #[test]
    fn invalid_clock_is_rejected_and_before_start_has_no_instance() {
        assert!(parse_clock("24:00").is_err());
        assert!(parse_clock("9:00").is_err());
        assert!(on_date(
            &daily("21:00"),
            parse_date("2025-12-31").unwrap(),
            &chrono::Utc
        )
        .is_none());
    }
}
