//! Attendance repository for sync and reporting operations.

use crate::entities::{attendance_logs, prelude::*};
use crate::models::attendance::{CreateAttendanceLog, DailyAttendance};
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::sea_query::OnConflict;
use sea_orm::*;

/// Insert a batch of attendance logs with deduplication.
///
/// Uses ON CONFLICT DO NOTHING to skip duplicates based on (device_uid, check_time).
/// Returns the count of successfully inserted records.
pub async fn insert_batch(
    db: &DatabaseConnection,
    records: &[CreateAttendanceLog],
) -> Result<usize, DbErr> {
    let mut inserted = 0;

    for record in records {
        let model = attendance_logs::ActiveModel {
            device_uid: Set(record.device_uid),
            check_time: Set(record.check_time.into()),
            verify_type: Set(record.verify_type),
            status: Set(record.status),
            source: Set(record.source.clone()),
            ..Default::default()
        };

        let result = AttendanceLogs::insert(model)
            .on_conflict(
                OnConflict::columns([
                    attendance_logs::Column::DeviceUid,
                    attendance_logs::Column::CheckTime,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(db)
            .await;

        if result.is_ok() {
            inserted += 1;
        }
    }

    Ok(inserted)
}

/// Insert a single attendance log.
///
/// Returns None if the record already exists (duplicate).
pub async fn insert_one(
    db: &DatabaseConnection,
    record: &CreateAttendanceLog,
) -> Result<Option<attendance_logs::Model>, DbErr> {
    let model = attendance_logs::ActiveModel {
        device_uid: Set(record.device_uid),
        check_time: Set(record.check_time.into()),
        verify_type: Set(record.verify_type),
        status: Set(record.status),
        source: Set(record.source.clone()),
        ..Default::default()
    };

    match model.insert(db).await {
        Ok(inserted) => Ok(Some(inserted)),
        Err(DbErr::RecordNotInserted) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Get attendance logs within a date range.
pub async fn get_by_date_range(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<attendance_logs::Model>, DbErr> {
    let start = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

    AttendanceLogs::find()
        .filter(attendance_logs::Column::CheckTime.between(start, end))
        .order_by_desc(attendance_logs::Column::CheckTime)
        .all(db)
        .await
}

/// Get attendance logs for a specific device within a date range.
pub async fn get_by_device_uid(
    db: &DatabaseConnection,
    device_uid: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<attendance_logs::Model>, DbErr> {
    let start = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

    AttendanceLogs::find()
        .filter(attendance_logs::Column::DeviceUid.eq(device_uid))
        .filter(attendance_logs::Column::CheckTime.between(start, end))
        .order_by_desc(attendance_logs::Column::CheckTime)
        .all(db)
        .await
}

/// Get daily attendance summary from the view.
pub async fn get_daily_summary(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<DailyAttendance>, DbErr> {
    DailyAttendance::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        SELECT
            employee_id,
            employee_code,
            full_name,
            department_id,
            department_name,
            work_date,
            first_check,
            last_check,
            check_count,
            work_hours
        FROM app.v_daily_attendance
        WHERE work_date BETWEEN $1 AND $2
        ORDER BY work_date DESC, employee_code
        "#,
        [start_date.into(), end_date.into()],
    ))
    .all(db)
    .await
}

/// Get daily attendance summary filtered by department.
pub async fn get_daily_summary_by_department(
    db: &DatabaseConnection,
    department_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<DailyAttendance>, DbErr> {
    DailyAttendance::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        SELECT
            employee_id,
            employee_code,
            full_name,
            department_id,
            department_name,
            work_date,
            first_check,
            last_check,
            check_count,
            work_hours
        FROM app.v_daily_attendance
        WHERE department_id = $1 AND work_date BETWEEN $2 AND $3
        ORDER BY work_date DESC, employee_code
        "#,
        [department_id.into(), start_date.into(), end_date.into()],
    ))
    .all(db)
    .await
}

/// Get the latest check time for incremental sync.
pub async fn get_latest_check_time(db: &DatabaseConnection) -> Result<Option<DateTime<Utc>>, DbErr> {
    let result = AttendanceLogs::find()
        .order_by_desc(attendance_logs::Column::CheckTime)
        .one(db)
        .await?;

    Ok(result.map(|r| r.check_time.with_timezone(&Utc)))
}

/// Get count of unique employees who checked in today.
pub async fn get_today_count(db: &DatabaseConnection) -> Result<u64, DbErr> {
    use sea_orm::sea_query::Expr;

    let today = chrono::Utc::now().date_naive();
    let start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = today.and_hms_opt(23, 59, 59).unwrap().and_utc();

    let result: Option<i64> = AttendanceLogs::find()
        .filter(attendance_logs::Column::CheckTime.between(start, end))
        .select_only()
        .column_as(
            Expr::col(attendance_logs::Column::DeviceUid).count_distinct(),
            "count",
        )
        .into_tuple()
        .one(db)
        .await?;

    Ok(result.unwrap_or(0) as u64)
}

/// Delete attendance logs before a given date.
pub async fn delete_before(db: &DatabaseConnection, before_date: NaiveDate) -> Result<u64, DbErr> {
    let before = before_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

    let result = AttendanceLogs::delete_many()
        .filter(attendance_logs::Column::CheckTime.lt(before))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Get total attendance log count.
pub async fn count_all(db: &DatabaseConnection) -> Result<u64, DbErr> {
    AttendanceLogs::find().count(db).await
}
