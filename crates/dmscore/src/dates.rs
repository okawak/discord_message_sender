use super::models::LocalDateTime;
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Timelike};

fn parse(timestamp: &str) -> Result<DateTime<chrono::FixedOffset>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| format!("Invalid Discord message timestamp: \"{timestamp}\"."))
}
fn format(date: NaiveDateTime) -> LocalDateTime {
    let year = date.year();
    // Match Chrono's ISO year representation, including BCE and expanded years.
    let year = if (0..10_000).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{year:+05}")
    };
    let month = date.month();
    let day = date.day();
    let hour = date.hour();
    let minute = date.minute();
    let second = date.second() + date.nanosecond() / 1_000_000_000;
    LocalDateTime {
        date: format!("{year}-{month:02}-{day:02}"),
        month: format!("{year}-{month:02}"),
        week: format!("{}-W{:02}", date.iso_week().year(), date.iso_week().week()),
        time: format!("{hour:02}:{minute:02}"),
        file_timestamp: format!("{year}{month:02}{day:02}_{hour:02}{minute:02}{second:02}"),
    }
}

pub fn local(timestamp: &str, zone: &str) -> Result<LocalDateTime, String> {
    let date = parse(timestamp)?;
    Ok(format(local_datetime(date, zone)?))
}

#[cfg(not(target_arch = "wasm32"))]
fn local_datetime(
    date: DateTime<chrono::FixedOffset>,
    zone: &str,
) -> Result<NaiveDateTime, String> {
    let zone: chrono_tz::Tz = zone
        .parse()
        .map_err(|_| format!("Invalid time zone: {zone}"))?;
    Ok(date.with_timezone(&zone).naive_local())
}

#[cfg(target_arch = "wasm32")]
use host::local_datetime;

pub fn possible_dates(timestamp: &str) -> Result<Vec<LocalDateTime>, String> {
    let date = parse(timestamp)?.naive_utc();
    [-12, 0, 14]
        .into_iter()
        .map(|hours| {
            date.checked_add_signed(Duration::hours(hours))
                .map(format)
                .ok_or_else(|| "Timestamp is out of range.".into())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn invalid_dates_and_time_zones_return_errors() {
        assert!(local("invalid", "UTC").is_err());
        assert!(local("2026-06-30T00:00:00Z", "invalid").is_err());
        assert!(possible_dates("invalid").is_err());
    }

    #[test]
    fn fixed_formats_match_chrono_across_calendar_boundaries() {
        for year in [
            -262_000, -10_000, -1, 0, 1, 999, 1900, 2000, 2026, 9999, 10_000, 262_000,
        ] {
            for month in 1..=12 {
                for day in [1, 28, 29, 30, 31] {
                    let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                        continue;
                    };
                    for (hour, minute, second, nanos) in [
                        (0, 0, 0, 0),
                        (9, 5, 7, 999_999_999),
                        (23, 59, 59, 1_999_999_999),
                    ] {
                        let date = date.and_hms_nano_opt(hour, minute, second, nanos).unwrap();
                        assert_eq!(
                            format(date),
                            LocalDateTime {
                                date: date.format("%Y-%m-%d").to_string(),
                                month: date.format("%Y-%m").to_string(),
                                week: format!(
                                    "{}-W{:02}",
                                    date.iso_week().year(),
                                    date.iso_week().week()
                                ),
                                time: date.format("%H:%M").to_string(),
                                file_timestamp: date.format("%Y%m%d_%H%M%S").to_string(),
                            },
                            "{date}"
                        );
                    }
                }
            }
        }
    }
}

// The JavaScript host already provides IANA time-zone rules through Intl.
// Only the civil-time conversion crosses this boundary; parsing and formatting stay in Rust.
#[cfg(target_arch = "wasm32")]
mod host {
    use super::*;
    use chrono::NaiveDate;
    use js_sys::Intl::{
        DateTimeFormatOptions, DateTimeFormatPart, DateTimeFormatPartType, DayFormat, EraFormat,
        HourCycle, MonthFormat, NumericFormat, YearFormat,
    };
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;

    // js-sys's stable constructor lacks `catch`; catch invalid zones without unwinding past Rust.
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Intl, js_name = DateTimeFormat)]
        #[derive(Clone)]
        type Formatter;
        #[wasm_bindgen(constructor, js_namespace = Intl, js_class = DateTimeFormat, catch)]
        fn new(locale: &str, options: &DateTimeFormatOptions) -> Result<Formatter, JsValue>;
        #[wasm_bindgen(method, js_name = formatToParts, catch)]
        fn parts(this: &Formatter, date: &js_sys::Date) -> Result<js_sys::Array, JsValue>;
    }

    thread_local! {
        // A single entry keeps repeated syncs cheap without retaining every zone ever requested.
        static FORMATTER: RefCell<Option<(String, Formatter)>> = const { RefCell::new(None) };
    }

    fn formatter(zone: &str) -> Result<Formatter, String> {
        FORMATTER.with_borrow_mut(|cached| {
            if let Some((key, formatter)) = cached.as_ref()
                && key == zone
            {
                return Ok(formatter.clone());
            }
            let options = DateTimeFormatOptions::new();
            options.set_time_zone(zone);
            options.set_calendar("gregory");
            options.set_numbering_system("latn");
            options.set_hour_cycle(HourCycle::H23);
            options.set_era(EraFormat::Short);
            options.set_year(YearFormat::Numeric);
            options.set_month(MonthFormat::TwoDigit);
            options.set_day(DayFormat::TwoDigit);
            options.set_hour(NumericFormat::TwoDigit);
            options.set_minute(NumericFormat::TwoDigit);
            options.set_second(NumericFormat::TwoDigit);
            let formatter = Formatter::new("en-GB", &options)
                .map_err(|_| format!("Invalid time zone: {zone}"))?;
            *cached = Some((zone.into(), formatter.clone()));
            Ok(formatter)
        })
    }

    pub(super) fn local_datetime(
        date: DateTime<chrono::FixedOffset>,
        zone: &str,
    ) -> Result<NaiveDateTime, String> {
        let formatter = formatter(zone)?;
        // Whole seconds keep subsecond rounding and Chrono's leap-second representation intact.
        let instant = js_sys::Date::new(&JsValue::from_f64(date.timestamp() as f64 * 1000.0));
        let error = || format!("Failed to convert timestamp in time zone: {zone}");
        let parts = formatter.parts(&instant).map_err(|_| error())?;
        let mut fields = [None; 6];
        let mut before_common_era = false;
        for part in parts.iter() {
            let part: DateTimeFormatPart = part.unchecked_into();
            let value = String::from(part.value());
            let index = match part.type_() {
                DateTimeFormatPartType::Year => 0,
                DateTimeFormatPartType::Month => 1,
                DateTimeFormatPartType::Day => 2,
                DateTimeFormatPartType::Hour => 3,
                DateTimeFormatPartType::Minute => 4,
                DateTimeFormatPartType::Second => 5,
                DateTimeFormatPartType::Era => {
                    before_common_era = value == "BC";
                    continue;
                }
                _ => continue,
            };
            fields[index] = value.parse::<u32>().ok();
        }
        let [
            Some(year),
            Some(month),
            Some(day),
            Some(hour),
            Some(minute),
            Some(second),
        ] = fields
        else {
            return Err(error());
        };
        let year = if before_common_era {
            1 - year as i32
        } else {
            year as i32
        };
        NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|day| {
                day.and_hms_nano_opt(hour, minute, second, date.timestamp_subsec_nanos())
            })
            .ok_or_else(error)
    }
}
