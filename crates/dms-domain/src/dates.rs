use super::models::LocalDateTime;
use jiff::{Timestamp, civil::DateTime, tz::offset};
use regex_lite::Regex;
use std::sync::LazyLock;

// Keep accepting the RFC 3339 form used by Discord, rather than Jiff's broader
// Temporal grammar. Fractions are validated here and omitted from the file timestamps.
static DISCORD_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([0-9]{4}-[0-9]{2}-[0-9]{2}[Tt ][0-9]{2}:[0-9]{2}:[0-9]{2})(?:\.[0-9]+)?([Zz]|[+-](?:[01][0-9]|2[0-3]):[0-5][0-9])$",
    )
    .unwrap()
});

fn parse(timestamp: &str) -> Result<(Timestamp, bool), String> {
    let invalid = || format!("Invalid Discord message timestamp: \"{timestamp}\".");
    let parts = DISCORD_TIMESTAMP.captures(timestamp).ok_or_else(invalid)?;
    let [datetime, offset] = parts.extract().1;
    // Jiff maps :60 to :59. Keep the original second for existing file names.
    let instant = format!("{datetime}{offset}")
        .parse()
        .map_err(|_| invalid())?;
    Ok((instant, datetime.ends_with(":60")))
}

fn format(date: DateTime, leap_second: bool) -> LocalDateTime {
    let year = date.year();
    // Preserve the signed ISO year format used in existing paths.
    let year = if (0..10_000).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{year:+05}")
    };
    let month = date.month();
    let day = date.day();
    let hour = date.hour();
    let minute = date.minute();
    let second = date.second() + i8::from(leap_second);
    let week = date.date().iso_week_date();
    LocalDateTime {
        date: format!("{year}-{month:02}-{day:02}"),
        month: format!("{year}-{month:02}"),
        week: format!("{}-W{:02}", week.year(), week.week()),
        time: format!("{hour:02}:{minute:02}"),
        file_timestamp: format!("{year}{month:02}{day:02}_{hour:02}{minute:02}{second:02}"),
    }
}

pub fn local(timestamp: &str, zone: &str) -> Result<LocalDateTime, String> {
    let (instant, leap_second) = parse(timestamp)?;
    Ok(format(local_datetime(instant, zone)?, leap_second))
}

#[cfg(not(target_arch = "wasm32"))]
fn local_datetime(instant: Timestamp, zone: &str) -> Result<DateTime, String> {
    let zone = jiff::tz::TimeZone::get(zone).map_err(|_| format!("Invalid time zone: {zone}"))?;
    Ok(zone.to_offset(instant).to_datetime(instant))
}

#[cfg(target_arch = "wasm32")]
use host::local_datetime;

pub fn possible_dates(timestamp: &str) -> Result<Vec<LocalDateTime>, String> {
    let (instant, leap_second) = parse(timestamp)?;
    Ok([-12, 0, 14]
        .into_iter()
        .map(|hours| format(offset(hours).to_datetime(instant), leap_second))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_dates_and_time_zones_return_errors() {
        for timestamp in [
            "invalid",
            "2026-02-29T00:00:00Z",
            "2026-06-30T00:00Z",
            "2026-06-30T00:00:00+00",
            "2026-06-30T00:00:00+24:00",
            "2026-06-30T00:00:00Z[UTC]",
        ] {
            let error = format!("Invalid Discord message timestamp: \"{timestamp}\".");
            assert_eq!(local(timestamp, "UTC"), Err(error.clone()));
            assert_eq!(possible_dates(timestamp), Err(error));
        }
        assert_eq!(
            local("2026-06-30T00:00:00Z", "invalid"),
            Err("Invalid time zone: invalid".into()),
        );
    }

    #[test]
    fn rfc3339_dates_keep_existing_file_names() {
        for (timestamp, expected) in [
            ("1969-12-31t23:59:59.999999999123z", "19691231_235959"),
            ("2020-01-01 00:00:01+00:02", "20191231_235801"),
            ("0000-01-01T00:00:00+00:01", "-00011231_235900"),
            ("2017-01-01T08:59:60.25+09:00", "20161231_235960"),
        ] {
            assert_eq!(local(timestamp, "UTC").unwrap().file_timestamp, expected);
            assert_eq!(
                possible_dates(timestamp).unwrap()[1].file_timestamp,
                expected
            );
        }
    }
}

// The JavaScript host already provides IANA time-zone rules through Intl.
// Only the civil-time conversion crosses this boundary; parsing and formatting stay in Rust.
#[cfg(target_arch = "wasm32")]
mod host {
    use super::*;
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

    pub(super) fn local_datetime(instant: Timestamp, zone: &str) -> Result<DateTime, String> {
        let formatter = formatter(zone)?;
        let instant = js_sys::Date::new(&JsValue::from_f64(instant.as_millisecond() as f64));
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
            fields[index] = value.parse::<i16>().ok();
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
        let year = if before_common_era { 1 - year } else { year };
        DateTime::new(
            year,
            month as i8,
            day as i8,
            hour as i8,
            minute as i8,
            second as i8,
            0,
        )
        .map_err(|_| error())
    }
}
