use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use chrono_tz::Tz;

#[allow(dead_code)]
pub fn now_in_tz(tz_name: &str) -> DateTime<FixedOffset> {
    let tz: Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
    let now = Utc::now();
    now.with_timezone(&tz).fixed_offset()
}

#[allow(dead_code)]
pub fn today_in_tz(tz_name: &str) -> NaiveDate {
    now_in_tz(tz_name).date_naive()
}

#[allow(dead_code)]
pub fn naive_to_utc(naive: NaiveDate) -> DateTime<Utc> {
    naive.and_hms_opt(0, 0, 0).unwrap().and_utc()
}

#[allow(dead_code)]
pub fn utc_to_naive_utc(dt: DateTime<Utc>) -> NaiveDate {
    dt.date_naive()
}
